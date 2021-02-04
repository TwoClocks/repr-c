use std::ops::Not;

use crate::layout::{Annotation, Array, BuiltinType, Record, RecordField, Type, TypeLayout};
use crate::result::{err, Error, ErrorKind, Result};
use crate::target::Target;
use crate::util::BITS_PER_BYTE;
use crate::visitor::{
    visit_array, visit_builtin_type, visit_opaque_type, visit_record_field, visit_typedef, Visitor,
};

pub mod common;
mod msvc;
mod sysv_like;

pub fn compute_layout(target: Target, ty: &Type<()>) -> Result<Type<TypeLayout>> {
    pre_validate(ty)?;
    use Target::*;
    match target {
        | Aarch64PcWindowsMsvc
        | I586PcWindowsMsvc
        | I686PcWindowsMsvc
        | I686UnknownWindows
        | Thumbv7aPcWindowsMsvc
        | X86_64UnknownWindows
        | X86_64PcWindowsMsvc => msvc::compute_layout(target, ty),
        I686PcWindowsGnu | X86_64PcWindowsGnu => sysv_like::mingw::compute_layout(target, ty),
        _ => sysv_like::sysv::compute_layout(target, ty),
    }
}

fn pre_validate(ty: &Type<()>) -> Result<()> {
    let mut pv = PreValidator(vec![]);
    pv.visit_type(ty);
    match pv.0.pop() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

struct PreValidator(Vec<Error>);

impl Visitor<()> for PreValidator {
    fn visit_annotations(&mut self, a: &[Annotation]) {
        let mut num_pragma_packed = 0;
        for a in a {
            match a {
                Annotation::PragmaPack(_) => num_pragma_packed += 1,
                Annotation::AttrPacked => {}
                Annotation::Aligned(None) => {}
                Annotation::Aligned(Some(n)) => {
                    self.validate_alignment(*n);
                }
            }
        }
        if num_pragma_packed > 1 {
            self.0.push(err(ErrorKind::MultiplePragmaPackedAnnotations));
        }
    }

    fn visit_builtin_type(&mut self, bi: BuiltinType, ty: &Type<()>) {
        if ty.annotations.is_empty().not() {
            self.0.push(err(ErrorKind::AnnotatedBuiltinType));
        }
        visit_builtin_type(self, bi, ty);
    }

    fn visit_record_field(&mut self, field: &RecordField<()>, rt: &Record<()>, ty: &Type<()>) {
        match (field.bit_width, field.named) {
            (Some(0), true) => self.0.push(err(ErrorKind::NamedZeroSizeBitField)),
            (None, false) => self.0.push(err(ErrorKind::UnnamedRegularField)),
            _ => {}
        }
        for a in &field.annotations {
            if let Annotation::PragmaPack(_) = a {
                self.0.push(err(ErrorKind::PragmaPackedField));
            }
        }
        visit_record_field(self, field, rt, ty);
    }

    fn visit_typedef(&mut self, dst: &Type<()>, ty: &Type<()>) {
        for a in &dst.annotations {
            match a {
                Annotation::Aligned(_) => {}
                Annotation::PragmaPack(_) => self.0.push(err(ErrorKind::PackedTypedef)),
                Annotation::AttrPacked => self.0.push(err(ErrorKind::PackedTypedef)),
            }
        }
        visit_typedef(self, dst, ty);
    }

    fn visit_array(&mut self, at: &Array<()>, ty: &Type<()>) {
        if ty.annotations.is_empty().not() {
            self.0.push(err(ErrorKind::AnnotatedArray));
        }
        visit_array(self, at, ty);
    }

    fn visit_opaque_type(&mut self, layout: TypeLayout, ty: &Type<()>) {
        if ty.annotations.is_empty().not() {
            self.0.push(err(ErrorKind::AnnotatedOpaqueType));
        }
        if layout.size_bits % BITS_PER_BYTE != 0 {
            self.0.push(err(ErrorKind::SubByteSize));
        }
        self.validate_alignment(layout.field_alignment_bits);
        self.validate_alignment(layout.required_alignment_bits);
        visit_opaque_type(self, layout, ty);
    }
}

impl PreValidator {
    fn validate_alignment(&mut self, a: u64) {
        if a < BITS_PER_BYTE {
            self.0.push(err(ErrorKind::SubByteAlignment));
        }
        if a.is_power_of_two().not() {
            self.0.push(err(ErrorKind::PowerOfTwoAlignment));
        }
    }
}
