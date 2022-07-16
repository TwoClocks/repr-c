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

const FindTarget = struct {
    arch: ?Arch = null,
    model: ?*const Model = null,
    os: ?Os.Tag = null,
    abi: ?Abi = null,
};

pub fn main() !void {
    const gpa = general_purpose_allocator.allocator();
    defer if (general_purpose_allocator.deinit()) std.process.exit(1);

    const res = try std.ChildProcess.exec(.{ .allocator = gpa, .argv = &.{
        "rustc",
        "--print",
        "target-list",
    } });

    var targets = std.mem.tokenize(u8, res.stdout, "\n");
    while (targets.next()) |target| {
        var fill = FindTarget{};

        std.debug.print("{s}\n", .{target});
        var ittr = std.mem.tokenize(u8, target, "-");
        while (ittr.next()) |part| {
            if (std.ascii.eqlIgnoreCase(part, "unknown")) continue;
            searchArch(part, &fill);
            searchModel(part, &fill);
            searchOs(part, &fill);
            searchAbi(part, &fill);
        }
        // ok. see if we can make a useful target out of all of this.
        if (fill.arch) |arch| {
            const end_tag = fill.os orelse Os.Tag.other;
            const end_os = Os{
                .tag = end_tag,
                .version_range = Os.VersionRange.default(end_tag, arch),
            };
            const end_model = fill.model orelse Model.generic(arch);
            const end_abi = fill.abi orelse Abi.default(arch, end_os);

            const result = Target{
                .cpu = .{
                    .arch = arch,
                    .model = end_model,
                    .features = end_model.features,
                },
                .os = end_os,
                .abi = end_abi,
            };
            std.debug.print("\t{s}-{s}-{s}-{s}\n", .{
                @tagName(result.cpu.arch),
                result.cpu.model.name,
                @tagName(result.os.tag),
                @tagName(result.abi),
            });
        } else {
            std.debug.print("\tno arch for target\n", .{});
            continue;
        }
    }
    gpa.free(res.stdout);
    gpa.free(res.stderr);
}

fn searchAbi(text: []const u8, res: *FindTarget) void {
    if (res.abi != null) return;
    inline for (@typeInfo(Abi).Enum.fields) |fld| {
        if (std.ascii.eqlIgnoreCase(fld.name, text)) {
            res.abi = @intToEnum(Target.Abi, fld.value);
            return;
        }
    }
}

fn searchOs(text: []const u8, res: *FindTarget) void {
    if (res.os != null) return;
    if (std.ascii.eqlIgnoreCase("darwin", text)) {
        res.os = Os.Tag.macos;
        if (res.arch.? == .i386) {
            // i686 etc is ambgious. can be both 32 or 64 bit
            // but macos is only 64 bit.
            std.debug.print("\tswap.\n", .{});
            res.arch = Arch.x86_64;
        }
        return;
    }
    inline for (@typeInfo(Os.Tag).Enum.fields) |fld| {
        if (std.ascii.eqlIgnoreCase(fld.name, text)) {
            res.os = @intToEnum(Target.Os.Tag, fld.value);
            if (res.os.? == .macos and res.arch.? == .i386) {
                // i686 etc is ambgious. can be both 32 or 64 bit
                // but macos is only 64 bit.
                std.debug.print("\tswap.\n", .{});
                res.arch = Arch.x86_64;
            }
            return;
        }
    }
}

fn searchModel(text: []const u8, res: *FindTarget) void {
    if (res.model != null or res.arch == null) return;
    for (res.arch.?.allCpuModels()) |mod| {
        if (mod.llvm_name) |ll| {
            if (std.ascii.eqlIgnoreCase(ll, text)) {
                res.model = mod;
                return;
            }
        }
    }
}

fn searchArch(text: []const u8, res: *FindTarget) void {
    if (res.arch != null) return;
    // look for an exact match.
    if (std.meta.stringToEnum(Arch, text)) |a| {
        res.arch = a;
        return;
    }

    // rust targets like armebv7r-none-eabi have both the arch and the cpu
    // in the "target" part of the string. Walk the string backwards and see
    // if ther is a arch in there.

    var end = text.len - 1;
    // no arch less than 3 long
    while (end > 2) : (end -= 1) {
        if (std.meta.stringToEnum(Arch, text[0..end])) |a| {
            // fount it.
            res.arch = a;

            const maybeModel = text[end..];
            if (maybeModel.len <= 0) return;

            // search for CPU model in the end of the string.

            for (a.allCpuModels()) |mod| {
                const ll = mod.llvm_name orelse continue;
                if (std.ascii.eqlIgnoreCase(ll, maybeModel)) {
                    res.model = mod;
                    return;
                }
            }
            // hmm... maybe it's a feature name.
            res.model = searchFeatures(text, a);
            if (res.model == null) {
                // sometines only the end part is a feature
                // std.debug.print(" l:{s} ", .{maybeModel});
                res.model = searchFeatures(maybeModel, a);
            }
            return;
        }
    }

    if (res.arch == null) {
        // sometimes the first part of the triplet is a cpu model
        // like i686
        // because i386 comes before x86_64 in the enum,
        // i686 etc will default to 32-bit (i386)
        // it's ambgious.. so.. seems safe?
        for (std.enums.values(Arch)) |a| {
            for (a.allCpuModels()) |mod| {
                const ll = mod.llvm_name orelse continue;
                if (std.ascii.eqlIgnoreCase(text, ll)) {
                    res.arch = a;
                    res.model = mod;
                    return;
                }
            }
        }
    }
    return;
}

fn searchFeatures(text: []const u8, arch: Arch) ?*const Model {
    var ret: ?*const Model = null;
    for (arch.allFeaturesList()) |feat| {
        if (feat.llvm_name) |ll| {
            if (std.ascii.eqlIgnoreCase(ll, text)) {
                var max: usize = std.math.maxInt(usize);
                for (arch.allCpuModels()) |mod| {
                    var f_count: usize = 0;
                    if (hasFeature(mod, feat, arch, &f_count)) {
                        if (f_count < max) {
                            ret = mod;
                            max = f_count;
                        }
                    }
                }
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
        const ffs = @intCast(usize, @ctz(usize, self.current));
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
        ret += @popCount(usize, i);
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

test "while" {
    var c: usize = 0;
    while (c < 100) : (c += 1) {
        if (c == 22) break;
    }
    std.debug.assert(c == 22);
}
