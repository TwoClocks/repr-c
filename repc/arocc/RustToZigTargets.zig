// at attemp to convert between rust and zig targets.
// rustc must be in the path.
// compile with `zig build-exe RustToZigTgarets.zig

const std = @import("std");
const CrossTarget = std.zig.CrossTarget;
var general_purpose_allocator = std.heap.GeneralPurposeAllocator(.{}){};
const Target = std.Target;
const Arch = std.Target.Cpu.Arch;
const Cpu = std.Target.Cpu;
const Model = std.Target.Cpu.Model;
const Feature = std.Target.Cpu.Feature;
const Set = std.Target.Cpu.Feature.Set;
const Os = std.Target.Os;
const Abi = std.Target.Abi;
const eqlIgnoreCase = std.ascii.eqlIgnoreCase;

const FindTarget = struct {
    arch: ?Arch = null,
    model: ?*const Model = null,
    os: ?Os.Tag = null,
    abi: ?Abi = null,
};

pub fn main() !void {
    const gpa = general_purpose_allocator.allocator();
    defer if (general_purpose_allocator.deinit()) std.process.exit(1);
    const o_file = try std.fs.cwd().createFile("mapping.text", .{});
    defer o_file.close();
    const out = o_file.writer();

    const rustc = try std.ChildProcess.exec(.{ .allocator = gpa, .argv = &.{
        "rustc",
        "--print",
        "target-list",
    } });
    defer {
        gpa.free(rustc.stdout);
        gpa.free(rustc.stderr);
    }

    // make a unique list from rustc as well
    // has the hard-coded targets from repr-c
    var rust_targets = std.StringArrayHashMap(void).init(gpa);
    defer rust_targets.deinit();
    // mutiple rust targets can map to the same
    // zig target. Keep track of the mappings
    // so we can find and eliminate these rust
    // targets
    var mapping = std.StringArrayHashMap(std.ArrayList([]const u8)).init(gpa);
    defer mapping.deinit();

    var rust_t = std.mem.tokenize(u8, rustc.stdout, "\n");
    while (rust_t.next()) |target| {
        try rust_targets.put(target, {});
    }
    for (static_list) |target| {
        try rust_targets.put(target, {});
    }
    var it = rust_targets.iterator();
    while (it.next()) |entry| {
        const key = entry.key_ptr.*;
        var z_target = try gpa.alloc(u8, 128);
        var strm = std.io.fixedBufferStream(z_target);
        if (try resolveTarget(key, strm.writer())) {
            const ent = try mapping.getOrPut(strm.getWritten());
            if (!ent.found_existing) {
                ent.value_ptr.* = std.ArrayList([]const u8).init(gpa);
            } else {
                gpa.free(z_target);
            }
            try ent.value_ptr.append(key);
        } else {
            gpa.free(z_target);
            std.debug.print("no target for {s}\n", .{key});
        }
    }
    var k_itr = mapping.iterator();
    while (k_itr.next()) |entry| {
        const k = entry.key_ptr.*;
        const v = entry.value_ptr.*;
        if (v.items.len > 1) {
            std.debug.print("multi {s} : ", .{k});
            for (v.items) |i| {
                std.debug.print(" {s}", .{i});
            }
            std.debug.print("\n", .{});
        }

        for (v.items) |t| {
            // safe to output
            try out.print("{s}:{s}\n", .{ t, k });
            // std.debug.print("single {s} : {s}\n", .{ k.*, v.items[0] });
        }
        v.deinit();
        gpa.free(k);
    }
}
fn resolveTarget(target: []const u8, writer: anytype) !bool {
    var fill: FindTarget = .{};

    // std.debug.print("{s} || ", .{target});
    var ittr = std.mem.split(u8, target, "-");
    var last: []const u8 = undefined;
    while (ittr.next()) |part| {
        last = part;
    }
    if (eqlIgnoreCase(last, "none")) {
        fill.abi = .none;
    }
    ittr.reset();
    while (ittr.next()) |part| {
        if (eqlIgnoreCase(part, "unknown") or eqlIgnoreCase(part, "none")) continue;
        if (isUnknown(part)) {
            fill.arch = null;
            break;
        }
        searchArch(part, &fill);
        searchModel(part, &fill);
        searchOs(part, &fill);
        searchAbi(part, &fill);
    }
    // ok. see if we can make a useful target out of all of this.
    if (fill.arch) |arch| {
        fill.os = fill.os orelse Os.Tag.other;
        fill.model = fill.model orelse Model.baseline(arch);
        const end_os: Os = .{
            .tag = fill.os.?,
            .version_range = .{ .none = {} },
        };
        fill.abi = fill.abi orelse Abi.default(arch, end_os);
        try writer.print("{s}-{s}-{s}-{s}", .{
            @tagName(fill.arch.?),
            fill.model.?.name,
            @tagName(fill.os.?),
            @tagName(fill.abi.?),
        });
        return true;
    }
    return false;
}
fn isUnknown(text: []const u8) bool {
    for (unknown_zig) |u| {
        if (eqlIgnoreCase(text, u)) return true;
    }
    return false;
}

fn searchAbi(text: []const u8, res: *FindTarget) void {
    if (res.abi != null) return;
    res.abi = std.meta.stringToEnum(Abi, text);
}

fn searchOs(text: []const u8, res: *FindTarget) void {
    if (res.os != null) return;
    if (eqlIgnoreCase("darwin", text) or eqlIgnoreCase("macosx", text)) {
        res.os = .macos;
        // if (res.arch.? == .i386) {
        //     // i686 etc is ambgious. can be both 32 or 64 bit
        //     // but macos is only 64 bit.
        //     res.arch = Arch.x86_64;
        // }
    } else {
        res.os = std.meta.stringToEnum(Os.Tag, text);
    }
}

fn searchModel(raw: []const u8, res: *FindTarget) void {
    const text = if (eqlIgnoreCase(raw, "i386")) "x86" else raw;
    if (res.model != null or res.arch == null) return;
    for (res.arch.?.allCpuModels()) |mod| {
        if (mod.llvm_name) |ll| {
            if (eqlIgnoreCase(ll, text)) {
                res.model = mod;
                return;
            }
        }
    }
}

fn searchArch(raw: []const u8, res: *FindTarget) void {
    const text = if (eqlIgnoreCase(raw, "i386")) "x86" else raw;
    if (res.arch != null) return;
    // look for an exact match.
    if (std.meta.stringToEnum(Arch, text)) |a| {
        res.arch = a;
        return;
    }
    // sometimes the first part of the triplet is a cpu model
    // like i686
    // because i386 comes before x86_64 in the enum,
    // i686 etc will default to 32-bit (i386)
    // it's ambgious.. so.. seems safe?
    for (std.enums.values(Arch)) |a| {
        for (a.allCpuModels()) |mod| {
            const ll = mod.llvm_name orelse continue;
            if (eqlIgnoreCase(text, ll)) {
                res.arch = a;
                res.model = mod;
                return;
            }
        }
    }

    // rust targets like armebv7r-none-eabi have both the arch and the cpu
    // in the "arch" part of the string. Walk the string backwards and see
    // if ther is a arch in the front

    var end = text.len - 1;
    // no arch less than 3 long
    while (end > 2) : (end -= 1) {
        if (std.meta.stringToEnum(Arch, text[0..end])) |a| {
            // fount it.
            res.arch = a;

            var maybeModel = text[end..];
            if (maybeModel.len <= 0) return;
            // rust has targeets like mipsisa32r, which zig is misp32r
            // so trip the isa
            if (a == Arch.mips and std.mem.startsWith(u8, maybeModel, "isa")) {
                maybeModel = maybeModel[3..];
            } else if (Arch.isRISCV(a)) {
                // strip off leading "i" if exists
                // zig doesn't support expiremtnal ISA's ATM.
                if (maybeModel[0] == 'i') {
                    maybeModel = maybeModel[1..];
                }
                searchRiscV(maybeModel, a, res);
                return;
            }

            // search for CPU model in the end of the string.

            for (a.allCpuModels()) |mod| {
                const ll = mod.llvm_name orelse continue;
                if (eqlIgnoreCase(ll, maybeModel)) {
                    res.model = mod;
                    return;
                }
            }
            // sometines only the end part is a feature
            res.model = searchFeatures(maybeModel, a);
        }
    }
}
fn searchRiscV(sub: []const u8, arch: Arch, fill: *FindTarget) void {
    if (sub.len == 0 or eqlIgnoreCase(sub, "gc")) {
        fill.arch = arch;
        fill.model = Model.generic(arch);
        return;
    }
    var f_union: Feature.Set = std.mem.zeroes(Feature.Set);
    const allFeat = arch.allFeaturesList();
    const has64 = allFeat[@enumToInt(std.Target.riscv.Feature.@"64bit")];
    for (sub) |_, idx| {
        const check = sub[idx .. idx + 1];
        for (allFeat) |hay| {
            if (eqlIgnoreCase(hay.name, check) or
                eqlIgnoreCase(hay.llvm_name orelse "", check))
            {
                f_union.addFeature(hay.index);
            }
        }
    }
    std.debug.print("rv-{s}-{s} ", .{ @tagName(arch), sub });
    var iter = featureIndexIterator(f_union);
    while (iter.next()) |i| {
        std.debug.print("{s}:", .{allFeat[i].name});
    }
    var min_count: usize = std.math.maxInt(usize);
    for (arch.allCpuModels()) |mod| {
        if (arch == .riscv64 and !hasFeature(mod, has64, .riscv64, null)) continue;
        var m_union: Feature.Set = std.mem.zeroes(Feature.Set);
        featuresUnion(mod, arch, &m_union);
        var match = true;
        for (m_union.ints) |_, idx| {
            const mi = m_union.ints[idx];
            const fi = f_union.ints[idx];
            if (mi & fi != fi) {
                match = false;
                break;
            }
        }
        const f_count = count(m_union);
        if (f_count < min_count) {
            fill.arch = arch;
            fill.model = mod;
            min_count = f_count;
        }
    }
    std.debug.print("   {s} {s}\n", .{ @tagName(arch), fill.model.?.name });
}

fn searchFeatures(sub: []const u8, arch: Arch) ?*const Model {
    var ret: ?*const Model = null;
    // all the armeb cpu modles/features called "armxxxx" so strip the eb
    const m_arch = if (arch == Arch.armeb) Arch.arm else arch;
    const full = @tagName(m_arch);
    const flen = full.len;
    for (arch.allFeaturesList()) |feat| {
        const ll = feat.llvm_name orelse "";
        // a bunch of these names are like 'mips64r6' and sub is just '64rlr'
        // so strip off the arch name from the front if it exists
        const lloff = if (std.mem.startsWith(u8, ll, full)) flen else 0;
        const noff = if (std.mem.startsWith(u8, feat.name, full)) flen else 0;
        // std.debug.print("b:{s}:{s}:{s}:{s}:{s}\n", .{ sub, @tagName(arch), @tagName(m_arch), ll[lloff..], feat.name[noff..] });
        if (eqlIgnoreCase(ll[lloff..], sub) or
            eqlIgnoreCase(feat.name[noff..], sub))
        {
            // std.debug.print("a:{s}:{s}:{s}:{s}:{s}\n", .{ sub, @tagName(arch), @tagName(m_arch), ll[lloff..], feat.name[noff..] });
            var max: usize = std.math.maxInt(usize);
            for (arch.allCpuModels()) |mod| {
                var f_count: usize = 0;
                if (hasFeature(mod, feat, arch, &f_count)) {
                    if (f_count < max) {
                        ret = mod;
                        max = f_count;
                        // std.debug.print("set:{s}={s}\n", .{ sub, mod.llvm_name });
                    }
                }
            }
            if (ret == null) {
                std.debug.print("No cpu has feature {s}\n", .{sub});
            }
        }
    }
    return ret;
}
const FeatureIterator = struct {
    const Self = @This();
    set: *const Set,
    ints_indx: usize,
    current: usize,

    pub fn next(self: *Self) ?Set.Index {
        // no ffs, so ctz+1
        const ffs = @intCast(usize, @ctz(self.current));
        if (ffs >= @bitSizeOf(usize)) {
            // have we checked the whole array?
            if (self.ints_indx == 0) return null;
            // load the next one and try again
            self.ints_indx -= 1;
            self.current = self.set.ints[self.ints_indx];
            return @call(.{ .modifier = .always_tail }, next, .{self});
        }
        // clear the bit we're about to return
        self.current &= (self.current - 1);
        const ret = ffs + (@bitSizeOf(usize) * (self.ints_indx));
        return @intCast(Set.Index, ret);
    }
};

pub fn featureIndexIterator(set: Set) FeatureIterator {
    return FeatureIterator{
        .set = &set,
        .ints_indx = set.ints.len - 1,
        .current = set.ints[set.ints.len - 1],
    };
}

/// how many features in this set
pub fn count(set: Set) usize {
    var ret: usize = 0;
    for (set.ints) |i| {
        ret += @popCount(i);
    }
    return ret;
}
/// a union of all the features and their dependencies.
pub fn featuresUnion(model: *const Model, arch: Arch, result: *Feature.Set) void {
    // start empty
    std.mem.set(usize, &result.ints, 0);
    walkFeatures(model.features, arch, result);
}
fn walkFeatures(node: Feature.Set, arch: Arch, result: *Feature.Set) void {
    for (result.ints) |*item, idx| {
        item.* |= node.ints[idx];
    }
    var itr = featureIndexIterator(node);
    while (itr.next()) |f_index| {
        const next_node = arch.allFeaturesList()[f_index];
        if (!next_node.dependencies.isEmpty()) {
            // if there is a loop in this graph, this will
            // blow the stack. But 'zig targets' runs, and that
            // dumps the whole graph.
            walkFeatures(next_node.dependencies, arch, result);
        }
    }
}

pub fn hasFeature(model: *const Model, feature: Feature, arch: Arch, f_count: ?*usize) bool {
    if (model.features.isEnabled(feature.index)) return true;
    var feat_union: Feature.Set = undefined;
    featuresUnion(model, arch, &feat_union);
    if (f_count) |c| {
        c.* = count(feat_union);
    }
    return feat_union.isEnabled(feature.index);
}
// these Os and Abi are not supported by zig
// don't bother mapping
const unknown_zig = [_][]const u8{
    "kmc",
    "solid_asp3",
    "softfloat",
    "wrs",
    "vxworks",
    "redox",
    "fortanix",
    "illumos",
    "uclibc",
    "linuxkernel", // some day
    // "elf", // can't map obj formats to a target
    "uclibc", // to bad. this seems like a zig thing
    "gnullvm", // this makes no sense to me
    "uwp",
    "openwrt",
    "muslabi64",
    "gnu_ilp32",
    "netbsdelf",
    "androideabi",
    "rumprun",
    "uclibceabi",
    "uclibcgnueabi",
    "sim",
    "kernel",
    "gnuspe",
    "sony",
    "arm64", // no 64bit arm yet.
};
const static_list = [_][]const u8{
    "aarch64-apple-macosx",
    "aarch64-fuchsia",
    "aarch64-linux-android",
    "aarch64-pc-windows-msvc",
    "aarch64-unknown-freebsd",
    "aarch64-unknown-hermit",
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "aarch64-unknown-netbsd",
    "aarch64-unknown-none",
    "aarch64-unknown-openbsd",
    "aarch64-unknown-redox",
    "arm64-apple-ios",
    "arm64-apple-ios-macabi",
    "arm64-apple-tvos",
    "armebv7r-unknown-none-eabi",
    "armebv7r-unknown-none-eabihf",
    "arm-linux-androideabi",
    "arm-unknown-linux-gnueabi",
    "arm-unknown-linux-gnueabihf",
    "armv4t-unknown-linux-gnueabi",
    "armv5te-unknown-linux-gnueabi",
    "armv5te-unknown-linux-uclibcgnueabi",
    "armv6-unknown-freebsd-gnueabihf",
    "armv6-unknown-netbsdelf-eabihf",
    "armv7a-none-eabi",
    "armv7a-none-eabihf",
    "armv7-apple-ios",
    "armv7-none-linux-android",
    "armv7r-unknown-none-eabi",
    "armv7r-unknown-none-eabihf",
    "armv7s-apple-ios",
    "armv7-unknown-freebsd-gnueabihf",
    "armv7-unknown-linux-gnueabi",
    "armv7-unknown-linux-gnueabihf",
    "armv7-unknown-netbsdelf-eabihf",
    "avr-unknown-unknown",
    "hexagon-unknown-linux-musl",
    "i386-apple-ios",
    "i586-pc-windows-msvc",
    "i586-unknown-linux-gnu",
    "i586-unknown-linux-musl",
    "i686-apple-macosx",
    "i686-linux-android",
    "i686-pc-windows-gnu",
    "i686-pc-windows-msvc",
    "i686-unknown-freebsd",
    "i686-unknown-haiku",
    "i686-unknown-linux-gnu",
    "i686-unknown-linux-musl",
    "i686-unknown-netbsdelf",
    "i686-unknown-openbsd",
    "i686-unknown-windows",
    "mips64el-unknown-linux-gnuabi64",
    "mips64el-unknown-linux-musl",
    "mips64-unknown-linux-gnuabi64",
    "mips64-unknown-linux-musl",
    "mipsel-sony-psp",
    "mipsel-unknown-linux-gnu",
    "mipsel-unknown-linux-musl",
    "mipsel-unknown-linux-uclibc",
    "mipsel-unknown-none",
    "mipsisa32r6el-unknown-linux-gnu",
    "mipsisa32r6-unknown-linux-gnu",
    "mipsisa64r6el-unknown-linux-gnuabi64",
    "mipsisa64r6-unknown-linux-gnuabi64",
    "mips-unknown-linux-gnu",
    "mips-unknown-linux-musl",
    "mips-unknown-linux-uclibc",
    "msp430-none-elf",
    "powerpc64le-unknown-linux-gnu",
    "powerpc64le-unknown-linux-musl",
    "powerpc64-unknown-freebsd",
    "powerpc64-unknown-linux-gnu",
    "powerpc64-unknown-linux-musl",
    "powerpc-unknown-linux-gnu",
    "powerpc-unknown-linux-gnuspe",
    "powerpc-unknown-linux-musl",
    "powerpc-unknown-netbsd",
    "riscv32",
    "riscv32-unknown-linux-gnu",
    "riscv64",
    "riscv64-unknown-linux-gnu",
    "s390x-unknown-linux-gnu",
    "sparc64-unknown-linux-gnu",
    "sparc64-unknown-netbsd",
    "sparc64-unknown-openbsd",
    "sparc-unknown-linux-gnu",
    "sparcv9-sun-solaris",
    "thumbv4t-none-eabi",
    "thumbv6m-none-eabi",
    "thumbv7a-pc-windows-msvc",
    "thumbv7em-none-eabihf",
    "thumbv7em-none-eabi",
    "thumbv7m-none-eabi",
    "thumbv8m.base-none-eabi",
    "thumbv8m.main-none-eabihf",
    "thumbv8m.main-none-eabi",
    "wasm32-unknown-emscripten",
    "wasm32-unknown-unknown",
    "wasm32-wasi",
    "x86_64-apple-ios-macabi",
    "x86_64-apple-ios",
    "x86_64-apple-macosx",
    "x86_64-apple-tvos",
    "x86_64-elf",
    "x86_64-fuchsia",
    "x86_64-linux-android",
    "x86_64-pc-solaris",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-msvc",
    "x86_64-rumprun-netbsd",
    "x86_64-unknown-dragonfly",
    "x86_64-unknown-freebsd",
    "x86_64-unknown-haiku",
    "x86_64-unknown-hermit",
    "x86_64-unknown-l4re-uclibc",
    "x86_64-unknown-linux-gnux32",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "x86_64-unknown-netbsd",
    "x86_64-unknown-openbsd",
    "x86_64-unknown-redox",
    "x86_64-unknown-windows",
};

test "while" {
    var c: usize = 0;
    while (c < 100) : (c += 1) {
        if (c == 22) break;
    }
    std.debug.assert(c == 22);
}
test "enums" {
    var found = false;
    inline for (@typeInfo(Abi).Enum.fields) |fld| {
        std.debug.print("{s}\n", .{fld.name});
        if (eqlIgnoreCase(fld.name, "eabi")) found = true;
    }
    std.debug.assert(found);
    std.debug.assert(std.meta.stringToEnum(Abi, "eabi") == Abi.eabi);
}
