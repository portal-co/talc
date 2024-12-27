use std::ops::{Mul, Sub};

use generic_array::{ArrayLength, GenericArray};
use talc_common::{cdef, Cfg, CfgExt};
use typenum::{Cmp, IsLessOrEqual, True, UInt, Unsigned, B0, U1, U16, U2, U4, U8};
use waffle::{Block, FunctionBody, Operator, Type, Value};
use waffle_ast::add_op;
pub struct Proto64 {}
impl Cfg for Proto64 {
    const MEMORY64: bool = true;

    type BITS = typenum::U64;
}

pub fn trim<C: Cfg>(a: Value, x: Value, f: &mut FunctionBody, k: Block) -> [Value; 2] {
    let b = f.add_op(k, C::const_64(0xffffffffffffffff), &[], &[C::ty()]);
    let b = f.add_op(k, cdef!(C => Xor), &[b, x], &[C::ty()]);
    [
        f.add_op(k, cdef!(C => And), &[a, x], &[C::ty()]),
        f.add_op(k, cdef!(C => And), &[a, b], &[C::ty()]),
    ]
}
pub fn merge<C: Cfg>(a: [Value; 2], x: Value, f: &mut FunctionBody, k: Block) -> Value {
    let b = f.add_op(k, C::const_64(0xffffffffffffffff), &[], &[C::ty()]);
    let b = f.add_op(k, cdef!(C => Xor), &[b, x], &[C::ty()]);
    let r = [
        f.add_op(k, cdef!(C => And), &[a[0], x], &[C::ty()]),
        f.add_op(k, cdef!(C => And), &[a[1], b], &[C::ty()]),
    ];
    f.add_op(k, cdef!(C => Or), &r, &[C::ty()])
}
pub struct MaskPalette {
    pub xff: Value,
    pub xffff: Value,
    pub xffffffff: Value,
    pub zero32: Value,
}
pub trait SM: Copy {
    type Nest: SM;
    type TN: ArrayLength + Mul<U8, Output: Sub<Self::Bytes>>;
    type Bytes: ArrayLength;
    fn split(self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> [Self::Nest; 2];
    fn merge(n: [Self::Nest; 2], p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self;
    fn from_arr(a: generic_array::GenericArray<Value, Self::TN>) -> Self;
    fn to_arr(self) -> generic_array::GenericArray<Value, Self::TN>;
    fn to_bytes(
        self,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> generic_array::GenericArray<Value, Self::Bytes>;
    fn from_bytes(
        a: generic_array::GenericArray<Value, Self::Bytes>,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> Self;
    fn select(self, other: Self, v: Value, f: &mut FunctionBody, k: Block) -> Self {
        let this = self.to_arr();
        let other = other.to_arr();
        Self::from_arr(generic_array::GenericArray::from_iter(
            this.into_iter().zip(other.into_iter()).map(|(a, b)| {
                let ty = f.values[a].ty(&f.type_pool).unwrap();
                let v = f.add_op(k, Operator::Select, &[v, a, b], &[ty]);
                v
            }),
        ))
    }
    fn bitwise(self, other: Self, op: Operator, f: &mut FunctionBody, k: Block) -> Self {
        let this = self.to_arr();
        let other = other.to_arr();
        Self::from_arr(generic_array::GenericArray::from_iter(
            this.into_iter().zip(other.into_iter()).map(|(a, b)| {
                let ty = f.values[a].ty(&f.type_pool).unwrap();
                let v = f.add_op(k, op.clone(), &[a, b], &[ty]);
                v
            }),
        ))
    }
    fn lanes<Lanes: Unsigned + ArrayLength, Target: SM<Bytes: Mul<Lanes, Output = Self::Bytes>>>(
        self,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> generic_array::GenericArray<Target, Lanes> {
        let bytes = self.to_bytes(p, f, k);
        //SAFETY: same size
        let vals: generic_array::GenericArray<
            generic_array::GenericArray<Value, Target::Bytes>,
            Lanes,
        > = unsafe { std::mem::transmute_copy(&bytes) };
        return generic_array::GenericArray::from_iter(
            vals.into_iter().map(|x| Target::from_bytes(x, p, f, k)),
        );
    }
    fn from_lanes<
        Lanes: Unsigned + ArrayLength,
        Target: SM<Bytes: Mul<Lanes, Output = Self::Bytes>>,
    >(
        l: generic_array::GenericArray<Target, Lanes>,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> Self {
        let vals: generic_array::GenericArray<GenericArray<Value, Target::Bytes>, Lanes> =
            GenericArray::from_iter(l.into_iter().map(|a| a.to_bytes(p, f, k)));
        //SAFETY: same size
        let vals: GenericArray<Value, Self::Bytes> = unsafe { std::mem::transmute_copy(&vals) };
        Self::from_bytes(vals, p, f, k)
    }
}
pub trait SMOps: SM {
    fn add(self, other: Self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> (Value, Self);
    fn mul(self, other: Self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> (Self, Self);
    fn lift(v: Value, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self;
    fn none(p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self;
    fn neg(self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self {
        let n = Self::none(p, f, k);
        return n.mul(self, p, f, k).1
    }
    fn last_byte(self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Value {
        self.to_bytes(p, f, k).last().cloned().unwrap()
    }
    fn shl(self, p: &MaskPalette, f: &mut FunctionBody, k: Block, v: Self) -> (Self, Self);
    fn rol(self, p: &MaskPalette, f: &mut FunctionBody, k: Block, v: Self) -> Self {
        let (a, b) = self.shl(p, f, k, v);
        a.add(b, p, f, k).1
    }
}
macro_rules! sm_smops {
    (impl<$($gn:ident),*> SMOps for $ty:ty => $base:ty $(where $($a:tt)*)?) => {
        impl<$($gn : SMOps),*> SMOps for $ty $(where $($a)*)?{
            fn add(self, other: Self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> (Value,Self){
                let [this0,this1] = self.split(p,f,k);
                let [other0,other1] = other.split(p,f,k);
                let (a,b) = this0.add(other0,p,f,k);
                let a = <$base as SMOps>::lift(a,p,f,k);
                let (a,c) = a.add(this1,p,f,k);
                let (d,c) = c.add(other1,p,f,k);
                let a = f.add_op(k,Operator::I32Add,&[a,d],&[Type::I32]);
                return (a,Self::merge([b,c],p,f,k));
            }
            fn mul(self, other: Self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> (Self,Self){
                let [this0,this1] = self.split(p,f,k);
                let [other0,other1] = other.split(p,f,k);
                let (h00,l00) = this0.mul(other0,p,f,k);
                let (h01,l01) = this0.mul(other1,p,f,k);
                let (h10,l10) = this1.mul(other0,p,f,k);
                let (h11,l11) = this1.mul(other1,p,f,k);
                let a1 = h00;
                let (c1,a1) = l01.add(a1,p,f,k);
                let (c2,a1) = l10.add(a1,p,f,k);
                let c1 = f.add_op(k,Operator::I32Add,&[c1,c2],&[Type::I32]);
                let c1 = <$base as SMOps>::lift(c1,p,f,k);
                let l = Self::merge([a1,l00],p,f,k);
                let a2 = l11;
                let (c1,a2) = c1.add(a2,p,f,k);
                let (c2,a2) = h10.add(a2,p,f,k);
                let (c3,a2) = h01.add(a2,p,f,k);
                let c2 = f.add_op(k,Operator::I32Add,&[c2,c2],&[Type::I32]);
                let c1 = f.add_op(k,Operator::I32Add,&[c1,c2],&[Type::I32]);
                let c1 = <$base as SMOps>::lift(c1,p,f,k);
                let (_,h11) = c1.add(h11,p,f,k);
                let h = Self::merge([h11,a2],p,f,k);
                (h,l)
            }
            fn lift(v: Value, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self{
                let a = <$base as SMOps>::lift(v,p,f,k);
                let z = p.zero32;
                let b = <$base as SMOps>::lift(z,p,f,k);
                Self::merge([a,b],p,f,k)
            }
            fn none(p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self{
                let a = <$base as SMOps>::none(p,f,k);
                Self::merge([a,a],p,f,k)
            }
            fn shl(self, p: &MaskPalette, f: &mut FunctionBody, k: Block, v: Self) -> (Self,Self){
                let [this0,this1] = self.split(p,f,k);
                let [other0,other1] = v.split(p,f,k);
                let (a,c) = this0.shl(p,f,k,other0);
                let (b,d) = this1.shl(p,f,k,other0);
                let (z,v) = a.add(d,p,f,k);
                let z = <$base as SMOps>::lift(z,p,f,k);
                (
                    Self::merge([b,z],p,f,k),
                    Self::merge([c,v],p,f,k)
                )
            }
        }
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]

pub struct Smaller<T>(pub T);
impl SM for Smaller<Smaller<Smaller<Value>>> {
    type Nest = Self;

    type TN = U1;

    fn split(self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> [Self::Nest; 2] {
        [self; 2]
    }

    fn merge(n: [Self::Nest; 2], p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self {
        n[0]
    }

    fn from_arr(a: generic_array::GenericArray<Value, Self::TN>) -> Self {
        Smaller(Smaller(Smaller(a[0])))
    }
    fn to_arr(self) -> generic_array::GenericArray<Value, Self::TN> {
        generic_array::arr![self.0 .0 .0]
    }

    type Bytes = U1;

    fn to_bytes(
        self,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> generic_array::GenericArray<Value, Self::Bytes> {
        generic_array::arr![self.0 .0 .0]
    }

    fn from_bytes(
        a: generic_array::GenericArray<Value, Self::Bytes>,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> Self {
        Smaller(Smaller(Smaller(a[0])))
    }
}
impl SMOps for Smaller<Smaller<Smaller<Value>>> {
    fn add(self, other: Self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> (Value, Self) {
        let r = f.add_op(
            k,
            Operator::I64Add,
            &[self.0 .0 .0, other.0 .0 .0],
            &[Type::I64],
        );
        let c = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
        let c = f.add_op(k, Operator::I32ShrU, &[r, c], &[Type::I64]);
        let c = f.add_op(k, Operator::I64Eqz, &[c], &[Type::I32]);
        let c = f.add_op(k, Operator::I32Eqz, &[c], &[Type::I32]);
        return (c, Smaller(Smaller(Smaller(r))));
    }

    fn mul(self, other: Self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> (Self, Self) {
        let r = f.add_op(
            k,
            Operator::I64Mul,
            &[self.0 .0 .0, other.0 .0 .0],
            &[Type::I64],
        );
        let c = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
        let c = f.add_op(k, Operator::I32ShrU, &[r, c], &[Type::I64]);
        // let c = f.add_op(k,Operator::I32WrapI64,&[c],&[Type::I32]);
        return (Smaller(Smaller(Smaller(c))), Smaller(Smaller(Smaller(r))));
    }

    fn lift(v: Value, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self {
        let v = f.add_op(k, Operator::I32And, &[v, p.xff], &[Type::I32]);
        let v = f.add_op(k, Operator::I64ExtendI32U, &[v], &[Type::I64]);
        Smaller(Smaller(Smaller(v)))
    }

    fn none(p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self {
        Smaller(Smaller(Smaller(p.xff)))
    }

    fn shl(self, p: &MaskPalette, f: &mut FunctionBody, k: Block, v: Self) -> (Self, Self) {
        let v = f.add_op(k, Operator::I32WrapI64, &[v.0 .0 .0], &[Type::I32]);
        (
            Smaller(Smaller(Smaller(f.add_op(
                k,
                Operator::I32ShrU,
                &[self.0 .0 .0, v],
                &[Type::I64],
            )))),
            Smaller(Smaller(Smaller(f.add_op(
                k,
                Operator::I32Shl,
                &[self.0 .0 .0, v],
                &[Type::I64],
            )))),
        )
    }
}
impl SM for Smaller<Smaller<Value>> {
    type Nest = Smaller<Self>;

    type TN = U1;

    fn split(self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> [Self::Nest; 2] {
        crate::trim::<Proto64>(self.0 .0, p.xff, f, k).map(|a| Smaller(Smaller(Smaller(a))))
    }

    fn merge(n: [Self::Nest; 2], p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self {
        Smaller(Smaller(crate::merge::<Proto64>(
            n.map(|a| a.0 .0 .0),
            p.xff,
            f,
            k,
        )))
    }

    fn from_arr(a: generic_array::GenericArray<Value, Self::TN>) -> Self {
        Smaller(Smaller(a[0]))
    }
    fn to_arr(self) -> generic_array::GenericArray<Value, Self::TN> {
        generic_array::arr![self.0 .0]
    }

    type Bytes = U2;

    fn to_bytes(
        self,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> generic_array::GenericArray<Value, Self::Bytes> {
        let s = self.split(p, f, k).map(|a| a.0 .0 .0);
        generic_array::arr![s[0], s[1]]
    }

    fn from_bytes(
        a: generic_array::GenericArray<Value, Self::Bytes>,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> Self {
        Self::merge([a[0], a[1]].map(|a| Smaller(Smaller(Smaller(a)))), p, f, k)
    }
}
sm_smops!(impl<> SMOps for Smaller<Smaller<Value>> => Smaller<Smaller<Smaller<Value>>>);
impl SM for Smaller<Value> {
    type Nest = Smaller<Self>;

    type TN = U1;

    fn split(self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> [Self::Nest; 2] {
        crate::trim::<Proto64>(self.0, p.xffff, f, k).map(|a| Smaller(Smaller(a)))
    }

    fn merge(n: [Self::Nest; 2], p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self {
        Smaller(crate::merge::<Proto64>(n.map(|a| a.0 .0), p.xffff, f, k))
    }

    fn from_arr(a: generic_array::GenericArray<Value, Self::TN>) -> Self {
        Smaller(a[0])
    }
    fn to_arr(self) -> generic_array::GenericArray<Value, Self::TN> {
        generic_array::arr![self.0]
    }

    type Bytes = U4;

    fn to_bytes(
        self,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> generic_array::GenericArray<Value, Self::Bytes> {
        let s = self.split(p, f, k);
        let s = s.map(|a| a.to_bytes(p, f, k));
        unsafe { std::mem::transmute_copy(&s) }
    }

    fn from_bytes(
        a: generic_array::GenericArray<Value, Self::Bytes>,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> Self {
        let s: [_; 2] = unsafe { std::mem::transmute_copy(&a) };
        let s = s.map(|a| <Self::Nest as SM>::from_bytes(a, p, f, k));
        Self::merge(s, p, f, k)
    }
}
sm_smops!(impl<> SMOps for Smaller<Value> => Smaller<Smaller<Value>>);
impl SM for Value {
    type Nest = Smaller<Self>;

    type TN = U1;

    fn split(self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> [Self::Nest; 2] {
        crate::trim::<Proto64>(self, p.xffffffff, f, k).map(|a| Smaller(a))
    }

    fn merge(n: [Self::Nest; 2], p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self {
        crate::merge::<Proto64>(n.map(|a| a.0), p.xffffffff, f, k)
    }

    fn from_arr(a: generic_array::GenericArray<Value, Self::TN>) -> Self {
        a[0]
    }
    fn to_arr(self) -> generic_array::GenericArray<Value, Self::TN> {
        generic_array::arr![self]
    }

    type Bytes = U8;

    fn to_bytes(
        self,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> generic_array::GenericArray<Value, Self::Bytes> {
        let s = self.split(p, f, k);
        let s = s.map(|a| a.to_bytes(p, f, k));
        unsafe { std::mem::transmute_copy(&s) }
    }

    fn from_bytes(
        a: generic_array::GenericArray<Value, Self::Bytes>,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> Self {
        let s: [_; 2] = unsafe { std::mem::transmute_copy(&a) };
        let s = s.map(|a| <Self::Nest as SM>::from_bytes(a, p, f, k));
        Self::merge(s, p, f, k)
    }
}
sm_smops!(impl<> SMOps for Value => Smaller<Value>);
impl<T: SM> SM for [T; 2]
where
    UInt<T::TN, B0>: Mul<U8, Output: Sub<UInt<T::Bytes, B0>>>,
{
    type Nest = T;

    type TN = UInt<T::TN, B0>;

    fn split(self, p: &MaskPalette, f: &mut FunctionBody, k: Block) -> [Self::Nest; 2] {
        self
    }

    fn merge(n: [Self::Nest; 2], p: &MaskPalette, f: &mut FunctionBody, k: Block) -> Self {
        n
    }

    fn from_arr(a: generic_array::GenericArray<Value, Self::TN>) -> Self {
        //SAFETY: same layout
        let v: [generic_array::GenericArray<Value, T::TN>; 2] =
            unsafe { std::mem::transmute_copy(&a) };
        v.map(T::from_arr)
    }

    fn to_arr(self) -> generic_array::GenericArray<Value, Self::TN> {
        let v = self.map(T::to_arr);
        //SAFETY: same layout
        unsafe { std::mem::transmute_copy(&v) }
    }

    type Bytes = UInt<T::Bytes, B0>;

    fn to_bytes(
        self,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> generic_array::GenericArray<Value, Self::Bytes> {
        let s = self.split(p, f, k);
        let s = s.map(|a| a.to_bytes(p, f, k));
        unsafe { std::mem::transmute_copy(&s) }
    }

    fn from_bytes(
        a: generic_array::GenericArray<Value, Self::Bytes>,
        p: &MaskPalette,
        f: &mut FunctionBody,
        k: Block,
    ) -> Self {
        let s: [_; 2] = unsafe { std::mem::transmute_copy(&a) };
        let s = s.map(|a| <Self::Nest as SM>::from_bytes(a, p, f, k));
        Self::merge(s, p, f, k)
    }
}
sm_smops!(impl<T> SMOps for [T; 2] => T where UInt<T::TN,B0>: Mul<U8,Output: Sub<UInt<T::Bytes,B0>>>);
