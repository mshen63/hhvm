// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.

mod opcodes;

use ffi::{
    BumpSliceMut,
    Maybe::{self, *},
    Slice, Str,
};
use iterator::IterId;
use label::Label;
use local::{Local, LocalId};

pub use opcodes::Opcodes;

/// see runtime/base/repo-auth-type.h
pub type RepoAuthType<'arena> = Str<'arena>;

/// Export these publicly so consumers of hhbc_ast don't have to know the
/// internal details about the ffi.
pub use hhvm_hhbc_defs_ffi::ffi::{
    BareThisOp, CollectionType, ContCheckOp, FCallArgsFlags, FatalOp, IncDecOp, InitPropOp,
    IsLogAsDynamicCallOp, IsTypeOp, MOpMode, OODeclExistsOp, ObjMethodOp, QueryMOp, ReadonlyOp,
    SetOpOp, SetRangeOp, SilenceOp, SpecialClsRef, SwitchKind, TypeStructResolveOp,
};

#[derive(Clone, Debug)]
#[repr(C)]
pub enum ParamId<'arena> {
    ParamUnnamed(isize),
    ParamNamed(Str<'arena>),
}

pub type StackIndex = u32;
pub type ClassNum = u32;

pub type ClassId<'arena> = hhbc_id::class::ClassType<'arena>;
pub type FunctionId<'arena> = hhbc_id::function::FunctionType<'arena>;
pub type MethodId<'arena> = hhbc_id::method::MethodType<'arena>;
pub type ConstId<'arena> = hhbc_id::constant::ConstType<'arena>;
pub type PropId<'arena> = hhbc_id::prop::PropType<'arena>;

pub type NumParams = u32;
pub type ByRefs<'arena> = Slice<'arena, bool>;

#[derive(Clone, Debug)]
#[repr(C)]
pub struct FcallArgs<'arena> {
    pub flags: FCallArgsFlags,
    pub num_args: NumParams,
    pub num_rets: NumParams,
    pub inouts: ByRefs<'arena>,
    pub readonly: ByRefs<'arena>,
    pub async_eager_target: Maybe<Label>,
    pub context: Maybe<Str<'arena>>,
}

impl<'arena> FcallArgs<'arena> {
    pub fn new(
        flags: FCallArgsFlags,
        num_rets: NumParams,
        num_args: NumParams,
        inouts: Slice<'arena, bool>,
        readonly: Slice<'arena, bool>,
        async_eager_target: Option<Label>,
        context: Option<&'arena str>,
    ) -> FcallArgs<'arena> {
        assert!(
            (inouts.is_empty() || inouts.len() == num_args as usize)
                && (readonly.is_empty() || readonly.len() == num_args as usize),
            "length of by_refs must be either zero or num_args"
        );
        FcallArgs {
            flags,
            num_args,
            num_rets,
            inouts,
            readonly,
            async_eager_target: async_eager_target.into(),
            context: context.map(|s| Str::new(s.as_bytes())).into(),
        }
    }

    pub fn targets(&self) -> &[Label] {
        match &self.async_eager_target {
            Just(x) => std::slice::from_ref(x),
            Nothing => &[],
        }
    }

    pub fn targets_mut(&mut self) -> &mut [Label] {
        match &mut self.async_eager_target {
            Just(x) => std::slice::from_mut(x),
            Nothing => &mut [],
        }
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct IterArgs<'arena> {
    pub iter_id: IterId,
    pub key_id: Maybe<Local<'arena>>,
    pub val_id: Local<'arena>,
}

pub type ClassrefId = isize;
/// Conventionally this is "A_" followed by an integer
pub type AdataId<'arena> = Str<'arena>;
pub type ParamLocations<'arena> = Slice<'arena, isize>;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum MemberKey<'arena> {
    EC(StackIndex, ReadonlyOp),
    EL(Local<'arena>, ReadonlyOp),
    ET(Str<'arena>, ReadonlyOp),
    EI(i64, ReadonlyOp),
    PC(StackIndex, ReadonlyOp),
    PL(Local<'arena>, ReadonlyOp),
    PT(PropId<'arena>, ReadonlyOp),
    QT(PropId<'arena>, ReadonlyOp),
    W,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum HasGenericsOp {
    NoGenerics,
    MaybeGenerics,
    HasGenerics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum ClassishKind {
    Class, // c.f. ast_defs::ClassishKind - may need Abstraction (Concrete, Abstract)
    Interface,
    Trait,
    Enum,
    EnumClass,
}
impl std::convert::From<oxidized::ast_defs::ClassishKind> for ClassishKind {
    fn from(k: oxidized::ast_defs::ClassishKind) -> Self {
        use oxidized::ast_defs;
        match k {
            ast_defs::ClassishKind::Cclass(_) => Self::Class,
            ast_defs::ClassishKind::Cinterface => Self::Interface,
            ast_defs::ClassishKind::Ctrait => Self::Trait,
            ast_defs::ClassishKind::Cenum => Self::Enum,
            ast_defs::ClassishKind::CenumClass(_) => Self::EnumClass,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum Visibility {
    Private,
    Public,
    Protected,
    Internal,
}
impl std::convert::From<oxidized::ast_defs::Visibility> for Visibility {
    fn from(k: oxidized::ast_defs::Visibility) -> Self {
        use oxidized::ast_defs;
        match k {
            ast_defs::Visibility::Private => Self::Private,
            ast_defs::Visibility::Public => Self::Public,
            ast_defs::Visibility::Protected => Self::Protected,
            ast_defs::Visibility::Internal => Self::Internal,
        }
    }
}
impl AsRef<str> for Visibility {
    fn as_ref(&self) -> &str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Internal => "internal",
        }
    }
}

/// A Contiguous range of unnamed locals. The canonical (default) empty
/// range is {0, 0}. Ranges of named locals cannot be represented.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LocalRange {
    pub start: LocalId,
    pub len: u32,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct SrcLoc {
    pub line_begin: isize,
    pub col_begin: isize,
    pub line_end: isize,
    pub col_end: isize,
}

/// These are HHAS pseudo-instructions that are handled in the HHAS parser and
/// do not have HHBC opcodes equivalents.
#[derive(Clone, Debug)]
#[repr(C)]
pub enum Pseudo<'arena> {
    Break(isize),
    Comment(Str<'arena>),
    Continue(isize),
    Label(Label),
    SrcLoc(SrcLoc),
    TryCatchBegin,
    TryCatchEnd,
    TryCatchMiddle,
    /// Pseudo instruction that will get translated into appropraite literal
    /// bytecode, with possible reference to .adata *)
    TypedValue(runtime::TypedValue<'arena>),
}

pub trait Targets {
    /// Return a slice of labels for the conditional branch targets of this
    /// instruction. This excludes the Label in an ILabel instruction, which is
    /// not a conditional branch.
    fn targets(&self) -> &[Label];

    /// Return a mutable slice of labels for the conditional branch targets of
    /// this instruction. This excludes the Label in an ILabel instruction,
    /// which is not a conditional branch.
    fn targets_mut(&mut self) -> &mut [Label];
}

#[derive(Clone, Debug)]
#[repr(C)]
pub enum Instruct<'arena> {
    // HHVM opcodes.
    Opcode(Opcodes<'arena>),
    // HHAS pseudo-instructions.
    Pseudo(Pseudo<'arena>),
}

impl Instruct<'_> {
    /// Return a slice of labels for the conditional branch targets of this instruction.
    /// This excludes the Label in an ILabel instruction, which is not a conditional branch.
    pub fn targets(&self) -> &[Label] {
        match self {
            Self::Opcode(opcode) => opcode.targets(),

            // Make sure new variants with branch target Labels are handled above
            // before adding items to this catch-all.
            Self::Pseudo(Pseudo::TypedValue(_))
            | Self::Pseudo(Pseudo::Continue(_))
            | Self::Pseudo(Pseudo::Break(_))
            | Self::Pseudo(Pseudo::Label(_))
            | Self::Pseudo(Pseudo::TryCatchBegin)
            | Self::Pseudo(Pseudo::TryCatchMiddle)
            | Self::Pseudo(Pseudo::TryCatchEnd)
            | Self::Pseudo(Pseudo::Comment(_))
            | Self::Pseudo(Pseudo::SrcLoc(_)) => &[],
        }
    }

    /// Return a mutable slice of labels for the conditional branch targets of this instruction.
    /// This excludes the Label in an ILabel instruction, which is not a conditional branch.
    pub fn targets_mut(&mut self) -> &mut [Label] {
        match self {
            Self::Opcode(opcode) => opcode.targets_mut(),

            // Make sure new variants with branch target Labels are handled above
            // before adding items to this catch-all.
            Self::Pseudo(Pseudo::TypedValue(_))
            | Self::Pseudo(Pseudo::Continue(_))
            | Self::Pseudo(Pseudo::Break(_))
            | Self::Pseudo(Pseudo::Label(_))
            | Self::Pseudo(Pseudo::TryCatchBegin)
            | Self::Pseudo(Pseudo::TryCatchMiddle)
            | Self::Pseudo(Pseudo::TryCatchEnd)
            | Self::Pseudo(Pseudo::Comment(_))
            | Self::Pseudo(Pseudo::SrcLoc(_)) => &mut [],
        }
    }
}
