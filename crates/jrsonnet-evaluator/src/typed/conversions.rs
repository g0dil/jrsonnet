use std::ops::Deref;

use jrsonnet_gcmodule::Cc;
use jrsonnet_interner::{IBytes, IStr};
pub use jrsonnet_macros::Typed;
use jrsonnet_types::{ComplexValType, ValType};

use crate::{
	error::Result,
	function::{native::NativeDesc, FuncDesc, FuncVal},
	throw,
	typed::CheckType,
	val::{ArrValue, IndexableVal},
	ObjValue, ObjValueBuilder, Val,
};

pub trait TypedObj: Typed {
	fn serialize(self, out: &mut ObjValueBuilder) -> Result<()>;
	fn parse(obj: &ObjValue) -> Result<Self>;
	fn into_object(self) -> Result<ObjValue> {
		let mut builder = ObjValueBuilder::new();
		self.serialize(&mut builder)?;
		Ok(builder.build())
	}
}

pub trait Typed: Sized {
	const TYPE: &'static ComplexValType;
	fn into_untyped(typed: Self) -> Result<Val>;
	fn from_untyped(untyped: Val) -> Result<Self>;
}

macro_rules! impl_int {
	($($ty:ty)*) => {$(
		impl Typed for $ty {
			const TYPE: &'static ComplexValType =
				&ComplexValType::BoundedNumber(Some(Self::MIN as f64), Some(Self::MAX as f64));
			fn from_untyped(value: Val) -> Result<Self> {
				<Self as Typed>::TYPE.check(&value)?;
				match value {
					Val::Num(n) => {
						#[allow(clippy::float_cmp)]
						if n.trunc() != n {
							throw!(
								"cannot convert number with fractional part to {}",
								stringify!($ty)
							)
						}
						Ok(n as Self)
					}
					_ => unreachable!(),
				}
			}
			fn into_untyped(value: Self) -> Result<Val> {
				Ok(Val::Num(value as f64))
			}
		}
	)*};
}

impl_int!(i8 u8 i16 u16 i32 u32);

macro_rules! impl_bounded_int {
	($($name:ident = $ty:ty)*) => {$(
		#[derive(Clone, Copy)]
		pub struct $name<const MIN: $ty, const MAX: $ty>($ty);
		impl<const MIN: $ty, const MAX: $ty> $name<MIN, MAX> {
			pub const fn new(value: $ty) -> Option<$name<MIN, MAX>> {
				if value >= MIN && value <= MAX {
					Some(Self(value))
				} else {
					None
				}
			}
			pub const fn value(self) -> $ty {
				self.0
			}
		}
		impl<const MIN: $ty, const MAX: $ty> Deref for $name<MIN, MAX> {
			type Target = $ty;
			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}

		impl<const MIN: $ty, const MAX: $ty> Typed for $name<MIN, MAX> {
			const TYPE: &'static ComplexValType =
				&ComplexValType::BoundedNumber(
					Some(MIN as f64),
					Some(MAX as f64),
				);

			fn from_untyped(value: Val) -> Result<Self> {
				<Self as Typed>::TYPE.check(&value)?;
				match value {
					Val::Num(n) => {
						#[allow(clippy::float_cmp)]
						if n.trunc() != n {
							throw!(
								"cannot convert number with fractional part to {}",
								stringify!($ty)
							)
						}
						Ok(Self(n as $ty))
					}
					_ => unreachable!(),
				}
			}

			fn into_untyped(value: Self) -> Result<Val> {
				Ok(Val::Num(value.0 as f64))
			}
		}
	)*};
}

impl_bounded_int!(
	BoundedI8 = i8
	BoundedI16 = i16
	BoundedI32 = i32
	BoundedI64 = i64
	BoundedUsize = usize
);

impl Typed for f64 {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Num);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Num(value))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Num(n) => Ok(n),
			_ => unreachable!(),
		}
	}
}

pub struct PositiveF64(pub f64);
impl Typed for PositiveF64 {
	const TYPE: &'static ComplexValType = &ComplexValType::BoundedNumber(Some(0.0), None);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Num(value.0))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Num(n) => Ok(Self(n)),
			_ => unreachable!(),
		}
	}
}
impl Typed for usize {
	// It is possible to store 54 bits of precision in f64, but leaving u32::MAX here for compatibility
	const TYPE: &'static ComplexValType =
		&ComplexValType::BoundedNumber(Some(0.0), Some(u32::MAX as f64));

	fn into_untyped(value: Self) -> Result<Val> {
		if value > u32::MAX as Self {
			throw!("number is too large")
		}
		Ok(Val::Num(value as f64))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Num(n) => {
				#[allow(clippy::float_cmp)]
				if n.trunc() != n {
					throw!("cannot convert number with fractional part to usize")
				}
				Ok(n as Self)
			}
			_ => unreachable!(),
		}
	}
}

impl Typed for IStr {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Str);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Str(value))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Str(s) => Ok(s),
			_ => unreachable!(),
		}
	}
}

impl Typed for String {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Str);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Str(value.into()))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Str(s) => Ok(s.to_string()),
			_ => unreachable!(),
		}
	}
}

impl Typed for char {
	const TYPE: &'static ComplexValType = &ComplexValType::Char;

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Str(value.to_string().into()))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Str(s) => Ok(s.chars().next().unwrap()),
			_ => unreachable!(),
		}
	}
}

impl<T> Typed for Vec<T>
where
	T: Typed,
{
	const TYPE: &'static ComplexValType = &ComplexValType::ArrayRef(T::TYPE);

	fn into_untyped(value: Self) -> Result<Val> {
		let mut o = Vec::with_capacity(value.len());
		for i in value {
			o.push(T::into_untyped(i)?);
		}
		Ok(Val::Arr(o.into()))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Arr(a) => {
				let mut o = Self::with_capacity(a.len());
				for i in a.iter() {
					o.push(T::from_untyped(i?)?);
				}
				Ok(o)
			}
			_ => unreachable!(),
		}
	}
}

/// To be used in Vec<Any>
/// Regular Val can't be used here, because it has wrong `TryFrom::Error` type
#[derive(Clone)]
pub struct Any(pub Val);

impl Typed for Any {
	const TYPE: &'static ComplexValType = &ComplexValType::Any;

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(value.0)
	}

	fn from_untyped(value: Val) -> Result<Self> {
		Ok(Self(value))
	}
}

/// Specialization, provides faster `TryFrom<VecVal>` for Val
pub struct VecVal(pub Cc<Vec<Val>>);

impl Typed for VecVal {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Arr);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Arr(ArrValue::Eager(value.0)))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Arr(a) => Ok(Self(a.evaluated()?)),
			_ => unreachable!(),
		}
	}
}

/// Specialization
impl Typed for IBytes {
	const TYPE: &'static ComplexValType =
		&ComplexValType::ArrayRef(&ComplexValType::BoundedNumber(Some(0.0), Some(255.0)));

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Arr(ArrValue::Bytes(value)))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		if let Val::Arr(ArrValue::Bytes(bytes)) = value {
			return Ok(bytes);
		}
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Arr(a) => {
				let mut out = Vec::with_capacity(a.len());
				for e in a.iter() {
					let r = e?;
					out.push(u8::from_untyped(r)?);
				}
				Ok(out.as_slice().into())
			}
			_ => unreachable!(),
		}
	}
}

pub struct M1;
impl Typed for M1 {
	const TYPE: &'static ComplexValType = &ComplexValType::BoundedNumber(Some(-1.0), Some(-1.0));

	fn into_untyped(_: Self) -> Result<Val> {
		Ok(Val::Num(-1.0))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		Ok(Self)
	}
}

macro_rules! decl_either {
	($($name: ident, $($id: ident)*);*) => {$(
		#[derive(Clone)]
		pub enum $name<$($id),*> {
			$($id($id)),*
		}
		impl<$($id),*> Typed for $name<$($id),*>
		where
			$($id: Typed,)*
		{
			const TYPE: &'static ComplexValType = &ComplexValType::UnionRef(&[$($id::TYPE),*]);

			fn into_untyped(value: Self) -> Result<Val> {
				match value {$(
					$name::$id(v) => $id::into_untyped(v)
				),*}
			}

			fn from_untyped(value: Val) -> Result<Self> {
				$(
					if $id::TYPE.check(&value).is_ok() {
						$id::from_untyped(value).map(Self::$id)
					} else
				)* {
					<Self as Typed>::TYPE.check(&value)?;
					unreachable!()
				}
			}
		}
	)*}
}
decl_either!(
	Either1, A;
	Either2, A B;
	Either3, A B C;
	Either4, A B C D;
	Either5, A B C D E;
	Either6, A B C D E F;
	Either7, A B C D E F G
);
#[macro_export]
macro_rules! Either {
	($a:ty) => {Either1<$a>};
	($a:ty, $b:ty) => {Either2<$a, $b>};
	($a:ty, $b:ty, $c:ty) => {Either3<$a, $b, $c>};
	($a:ty, $b:ty, $c:ty, $d:ty) => {Either4<$a, $b, $c, $d>};
	($a:ty, $b:ty, $c:ty, $d:ty, $e:ty) => {Either5<$a, $b, $c, $d, $e>};
	($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty) => {Either6<$a, $b, $c, $d, $e, $f>};
	($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty) => {Either7<$a, $b, $c, $d, $e, $f, $g>};
}
pub use Either;

pub type MyType = Either![u32, f64, String];

impl Typed for ArrValue {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Arr);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Arr(value))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Arr(a) => Ok(a),
			_ => unreachable!(),
		}
	}
}

impl Typed for FuncVal {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Func);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Func(value))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Func(a) => Ok(a),
			_ => unreachable!(),
		}
	}
}

impl Typed for Cc<FuncDesc> {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Func);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Func(FuncVal::Normal(value)))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Func(FuncVal::Normal(desc)) => Ok(desc),
			Val::Func(_) => throw!("expected normal function, not builtin"),
			_ => unreachable!(),
		}
	}
}

impl Typed for ObjValue {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Obj);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Obj(value))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Obj(a) => Ok(a),
			_ => unreachable!(),
		}
	}
}

impl Typed for bool {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Bool);

	fn into_untyped(value: Self) -> Result<Val> {
		Ok(Val::Bool(value))
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		match value {
			Val::Bool(a) => Ok(a),
			_ => unreachable!(),
		}
	}
}
impl Typed for IndexableVal {
	const TYPE: &'static ComplexValType = &ComplexValType::UnionRef(&[
		&ComplexValType::Simple(ValType::Arr),
		&ComplexValType::Simple(ValType::Str),
	]);

	fn into_untyped(value: Self) -> Result<Val> {
		match value {
			IndexableVal::Str(s) => Ok(Val::Str(s)),
			IndexableVal::Arr(a) => Ok(Val::Arr(a)),
		}
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		value.into_indexable()
	}
}

pub struct Null;
impl Typed for Null {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Null);

	fn into_untyped(_: Self) -> Result<Val> {
		Ok(Val::Null)
	}

	fn from_untyped(value: Val) -> Result<Self> {
		<Self as Typed>::TYPE.check(&value)?;
		Ok(Self)
	}
}

pub struct NativeFn<D: NativeDesc>(D::Value);
impl<D: NativeDesc> Deref for NativeFn<D> {
	type Target = D::Value;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl<D: NativeDesc> Typed for NativeFn<D> {
	const TYPE: &'static ComplexValType = &ComplexValType::Simple(ValType::Func);

	fn into_untyped(_typed: Self) -> Result<Val> {
		throw!("can only convert functions from jsonnet to native")
	}

	fn from_untyped(untyped: Val) -> Result<Self> {
		Ok(Self(
			untyped
				.as_func()
				.expect("shape is checked")
				.into_native::<D>(),
		))
	}
}
