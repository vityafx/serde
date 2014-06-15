// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::hashmap;
use std::collections::treemap;
use std::collections::{HashMap, TreeMap};
use std::hash::Hash;
use std::iter;
use std::option;
use std::slice;

#[deriving(Clone, PartialEq, Show)]
pub enum Token<'a> {
    Null,
    Bool(bool),
    Int(int),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Uint(uint),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Char(char),
    Str(&'a str),
    Option(bool),

    TupleStart(uint),
    StructStart(&'a str, uint),
    EnumStart(&'a str, &'a str, uint),
    SeqStart(uint),
    MapStart(uint),

    End,
}

//////////////////////////////////////////////////////////////////////////////

/*
pub trait Serializer<'a>: Iterator<Token<'a>> {
    //fn serialize(&mut self), token: Token<'a>) -> Result<(), E>;
}
*/

//////////////////////////////////////////////////////////////////////////////

pub trait Serializable<'a, Iter: Iterator<Token<'a>>> {
    fn serialize(&'a self) -> Iter;
}

//////////////////////////////////////////////////////////////////////////////

macro_rules! impl_serializable {
    ($ty:ty, $variant:expr) => {
        impl<'a> Serializable<'a, option::Item<Token<'a>>> for $ty {
            #[inline]
            fn serialize(&'a self) -> option::Item<Token<'a>> {
                Some($variant).move_iter()
            }
        }
    }
}

impl_serializable!((), Null)
impl_serializable!(bool, Bool(*self))
impl_serializable!(int, Int(*self))
impl_serializable!(i8, I8(*self))
impl_serializable!(i16, I16(*self))
impl_serializable!(i32, I32(*self))
impl_serializable!(i64, I64(*self))
impl_serializable!(uint, Uint(*self))
impl_serializable!(u8, U8(*self))
impl_serializable!(u16, U16(*self))
impl_serializable!(u32, U32(*self))
impl_serializable!(u64, U64(*self))
impl_serializable!(f32, F32(*self))
impl_serializable!(f64, F64(*self))
impl_serializable!(char, Char(*self))
impl_serializable!(&'a str, Str(*self))
impl_serializable!(String, Str(self.as_slice()))

//////////////////////////////////////////////////////////////////////////////

/*
struct Tuple2Items<T> {
    idx: uint,
    tuple: (T, T),
}

impl<'a, T> Iterator<&'a T> for Tuple2Items<T> {
    fn next(&mut self) -> Option<&'a T> {
        match self.idx {
            0 => {
                self.idx += 1;
                Some(self.tuple.ref0)
            }
            1 => {
                self.idx += 1;
                Some(self.tuple.ref0)
            }
            _ => {
                None
            }
        }
    }
}
*/

//////////////////////////////////////////////////////////////////////////////

impl<
    'a,
    Iter: Iterator<Token<'a>>,
    T: Serializable<'a, Iter>
> Serializable<'a, OptionSerializer<'a, Iter>> for Option<T> {
    #[inline]
    fn serialize(&'a self) -> OptionSerializer<'a, Iter> {
        let iter: Option<Iter> = match *self {
            Some(ref value) => Some(value.serialize()),
            None => None,
        };

        OptionSerializer {
            start: true,
            iter: iter,
        }
    }
}

pub struct OptionSerializer<'a, Iter> {
    start: bool,
    iter: Option<Iter>,
}

impl<
    'a,
    Iter: Iterator<Token<'a>>
> Iterator<Token<'a>> for OptionSerializer<'a, Iter>{
    fn next(&mut self) -> Option<Token<'a>> {
        if self.start {
            self.start = false;
            Some(Option(self.iter.is_some()))
        } else {
            match self.iter {
                Some(ref mut iter) => iter.next(),
                None => None,
            }
        }
    }
}

//////////////////////////////////////////////////////////////////////////////

pub struct CompoundSerializer<'a, Iter> {
    start_token: Option<Token<'a>>,
    iter: Iter,
    finished: bool,
}

impl<'a, Iter: Iterator<Token<'a>>> CompoundSerializer<'a, Iter> {
    pub fn new(start_token: Token<'a>, iter: Iter) -> CompoundSerializer<'a, Iter> {
        CompoundSerializer {
            start_token: Some(start_token),
            iter: iter,
            finished: false,
        }
    }
}

impl<'a, Iter: Iterator<Token<'a>>> Iterator<Token<'a>> for CompoundSerializer<'a, Iter> {
    #[inline]
    fn next(&mut self) -> Option<Token<'a>> {
        if self.finished {
            None
        } else {
            match self.start_token.take() {
                Some(token) => { return Some(token); }
                None => { }
            }

            match self.iter.next() {
                Some(token) => { return Some(token); }
                None => {
                    self.finished = true;
                    Some(End)
                }
            }
        }
    }
}

//////////////////////////////////////////////////////////////////////////////

pub type SeqSerializer<'a, T, Iter, Items> =
    CompoundSerializer<
        'a,
        iter::FlatMap<
            'a,
            &'a T,
            Items,
            Iter
        >
    >;

impl<
    'a,
    T: Serializable<'a, Iter>,
    Iter: Iterator<Token<'a>>
> Serializable<
    'a,
    SeqSerializer<
        'a,
        T,
        Iter,
        slice::Items<'a, T>
    >
> for Vec<T> {
    #[inline]
    fn serialize(&'a self) -> SeqSerializer<
        'a,
        T,
        Iter,
        slice::Items<'a, T>
    > {
        CompoundSerializer::new(
            SeqStart(self.len()),
            self.iter().flat_map(|v| v.serialize())
        )
    }
}


//////////////////////////////////////////////////////////////////////////////

pub type MapSerializer<'a, K, V, KeyIter, ValIter, Items> =
    CompoundSerializer<
        'a,
        iter::FlatMap<
            'a,
            (&'a K, &'a V),
            Items,
            iter::Chain<
                KeyIter,
                ValIter
            >
        >
    >;

impl<
    'a,
    K: Serializable<'a, KeyIter> + Eq + Hash,
    V: Serializable<'a, ValIter>,
    KeyIter: Iterator<Token<'a>>,
    ValIter: Iterator<Token<'a>>
> Serializable<
    'a,
    MapSerializer<
        'a,
        K,
        V,
        KeyIter,
        ValIter,
        hashmap::Entries<'a, K, V>
    >
> for HashMap<K, V> {
    #[inline]
    fn serialize(&'a self) -> MapSerializer<
        'a,
        K,
        V,
        KeyIter,
        ValIter,
        hashmap::Entries<'a, K, V>
    > {
        CompoundSerializer::new(
            MapStart(self.len()),
            self.iter().flat_map(|(k, v)| k.serialize().chain(v.serialize()))
        )
    }
}

impl<
    'a,
    K: Serializable<'a, KeyIter> + Ord,
    V: Serializable<'a, ValIter>,
    KeyIter: Iterator<Token<'a>>,
    ValIter: Iterator<Token<'a>>
> Serializable<
    'a,
    MapSerializer<
        'a,
        K,
        V,
        KeyIter,
        ValIter,
        treemap::Entries<'a, K, V>
    >
> for TreeMap<K, V> {
    #[inline]
    fn serialize(&'a self) -> MapSerializer<
        'a,
        K,
        V,
        KeyIter,
        ValIter,
        treemap::Entries<'a, K, V>
    > {
        CompoundSerializer::new(
            MapStart(self.len()),
            self.iter().flat_map(|(k, v)| k.serialize().chain(v.serialize()))
        )
    }
}

/*
//////////////////////////////////////////////////////////////////////////////

macro_rules! peel {
    ($name:ident, $($other:ident,)*) => (impl_serialize_tuple!($($other,)*))
}

macro_rules! impl_serialize_tuple {
    () => {
        impl<
            E,
            S: Serializer<E>
        > Serializable<E, S> for () {
            #[inline]
            fn serialize(&self, s: &mut S) -> Result<(), E> {
                s.serialize(Null)
            }
        }
    };
    ( $($name:ident,)+ ) => {
        impl<
            E,
            S: Serializer<E>,
            $($name:Serializable<E, S>),*
        > Serializable<E, S> for ($($name,)*) {
            #[inline]
            #[allow(uppercase_variables)]
            fn serialize(&self, s: &mut S) -> Result<(), E> {
                // FIXME: how can we count macro args?
                let mut len = 0;
                $({ let $name = 1; len += $name; })*;

                let ($(ref $name,)*) = *self;

                try!(s.serialize(TupleStart(len)));
                $(try!($name.serialize(s));)*
                s.serialize(End)
            }
        }
        peel!($($name,)*)
    }
}

impl_serialize_tuple! { T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, }

//////////////////////////////////////////////////////////////////////////////
*/

//////////////////////////////////////////////////////////////////////////////

struct Empty;

impl<T> Iterator<T> for Empty {
    #[inline]
    fn next(&mut self) -> Option<T> {
        None
    }
}

//////////////////////////////////////////////////////////////////////////////

enum Variants2<T0, T1> {
    Variant0(T0),
    Variant1(T1),
}

impl<
    T,
    T0: Iterator<T>,
    T1: Iterator<T>
> Iterator<T> for Variants2<T0, T1> {
    fn next(&mut self) -> Option<T> {
        match *self {
            Variant0(ref mut iter) => iter.next(),
            Variant1(ref mut iter) => iter.next(),
        }
    }
}

/*
macro_rules! peel_iterator_variants {
    ($name:ident, $($other:ident,)*) => (impl_iterator_variants!($($other,)*))
}

macro_rules! impl_iterator_variants {
    () => { };
    ( $($idx:ident,)+ ) => {
        pub enum Variants<$($idx),*> {
            $(Variant($idx)),*
        }
        /*
        */

        /*
        impl<
            T //,
            //$($idx:Iterator<T>),*
        > Iterator<T> for ($(T$idx,)*) {
            #[inline]
            #[allow(uppercase_variables)]
            fn next(&mut self) -> Option<T> {
                match *self {
                    $(
                        Variant$idx(ref mut iter) => iter.next(),
                    ),*
                }
            }
        }
        */
        peel_iterator_variants!($($idx,)*)
    }
}

impl_iterator_variants! { V0, V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, }
*/

macro_rules! impl_iterator_variant {
    ($name:ident, $($variant:ident),*) => {
        #[allow(non_camel_case_types)]
        pub enum $name<$($variant),*> {
            $($variant($variant)),*
        }

        impl<
            T,
            $($variant:Iterator<T>),*
        > Iterator<T> for $name<$($variant),*> {
            #[inline]
            #[allow(uppercase_variables)]
            fn next(&mut self) -> Option<T> {
                match *self {
                    $( $variant(ref mut iter) => iter.next() ),*
                }
            }
        }
    }
}

impl_iterator_variant!(Enum1, Variant1_0)
impl_iterator_variant!(Enum2, Variant2_0, Variant2_1)
impl_iterator_variant!(Enum3, Variant3_0, Variant3_1, Variant3_2)

//////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::{Token, Null, Bool, Int, Uint, Char, Str, Option};
    use super::{SeqStart, MapStart, EnumStart, StructStart, End};
    use super::Serializable;
    use super::CompoundSerializer;
    use super::{OptionSerializer, SeqSerializer, MapSerializer};
    use super::{Empty, Enum2, Variant2_0, Variant2_1};

    use std::collections::hashmap;
    use std::collections::{HashMap, TreeMap};
    use std::iter;
    use std::option;
    use std::slice;

    macro_rules! treemap {
        ($($k:expr => $v:expr),*) => ({
            let mut _m = ::std::collections::TreeMap::new();
            $(_m.insert($k, $v);)*
            _m
        })
    }

    /*
    use std::collections::{HashMap, TreeMap};

    use serialize::Decoder;

    use super::{Token, Null, Int, Uint, Str, Char, Option};
    use super::{TupleStart, StructStart, EnumStart};
    use super::{SeqStart, MapStart, End};
    use super::{Serializer, Serializable};

    //////////////////////////////////////////////////////////////////////////////
    */

    #[deriving(Clone, PartialEq, Show)]
    struct Inner {
        a: (),
        b: uint,
        c: HashMap<String, Option<char>>,
    }

    impl<'a> Serializable<'a, InnerSerializer<'a>> for Inner {
        #[inline]
        fn serialize(&'a self) -> InnerSerializer<'a> {
            CompoundSerializer::new(
                StructStart("Inner", 3),
                Some(Str("a")).move_iter()
                    .chain(self.a.serialize())
                    .chain(Some(Str("b")).move_iter())
                    .chain(self.b.serialize())
                    .chain(Some(Str("c")).move_iter())
                    .chain(self.c.serialize())
            )
        }
    }

    type InnerSerializer<'a> =
        CompoundSerializer<
            'a,
            iter::Chain<
                iter::Chain<
                    iter::Chain<
                        iter::Chain<
                            iter::Chain<
                                option::Item<Token<'a>>,
                                option::Item<Token<'a>>
                            >,
                            option::Item<Token<'a>>
                        >,
                        option::Item<Token<'a>>
                    >,
                    option::Item<Token<'a>>
                >,
                MapSerializer<
                    'a,
                    String,
                    Option<char>,
                    option::Item<Token<'a>>,
                    OptionSerializer<
                        'a,
                        option::Item<Token<'a>>
                    >,
                    hashmap::Entries<
                        'a,
                        String,
                        Option<char>
                    >
                >
            >
        >;

    //////////////////////////////////////////////////////////////////////////////

    #[deriving(Clone, PartialEq, Show)]
    struct Outer {
        inner: Vec<Inner>,
    }

    impl<'a> Serializable<'a, OuterSerializer<'a>> for Outer {
        #[inline]
        fn serialize(&'a self) -> OuterSerializer<'a> {
            CompoundSerializer::new(
                StructStart("Outer", 1),
                Some(Str("inner")).move_iter()
                    .chain(self.inner.serialize())
            )
        }
    }

    type OuterSerializer<'a> =
        CompoundSerializer<
            'a,
            iter::Chain<
                option::Item<Token<'a>>,
                SeqSerializer<
                    'a,
                    Inner,
                    InnerSerializer<'a>,
                    slice::Items<'a, Inner>
                >
            >
        >;

    //////////////////////////////////////////////////////////////////////////////

    #[deriving(Clone, PartialEq, Show)]
    enum Animal {
        Dog,
        Frog(String, int)
    }

    impl<'a> Serializable<'a, AnimalSerializer<'a>> for Animal {
        #[inline]
        fn serialize(&'a self) -> AnimalSerializer<'a> {
            match *self {
                Dog => {
                    CompoundSerializer::new(
                        EnumStart("Animal", "Dog", 0),
                        Variant2_0(Empty)
                    )
                }
                Frog(ref x, ref y) => {
                    CompoundSerializer::new(
                        EnumStart("Animal", "Frog", 2),
                        Variant2_1(x.serialize().chain(y.serialize()))
                    )
                }
            }
        }
    }

    pub type AnimalSerializer<'a> =
        CompoundSerializer<
            'a,
            Enum2<
                Empty,
                iter::Chain<
                    option::Item<
                        Token<'a>
                    >,
                    option::Item<
                        Token<'a>
                    >
                >
            >
        >;

    /*
    //////////////////////////////////////////////////////////////////////////////

    #[deriving(Show)]
    enum Error {
        EndOfStream,
        SyntaxError,
    }

    //////////////////////////////////////////////////////////////////////////////

    struct AssertSerializer<Iter> {
        iter: Iter,
    }

    impl<'a, Iter: Iterator<Token<'a>>> AssertSerializer<Iter> {
        #[inline]
        fn new(iter: Iter) -> AssertSerializer<Iter> {
            AssertSerializer {
                iter: iter,
            }
        }
    }

    impl<'a, Iter: Iterator<Token<'a>>> Serializer<Error> for AssertSerializer<Iter> {
        #[inline]
        fn serialize<'b>(&mut self, token: Token<'b>) -> Result<(), Error> {
            let t = match self.iter.next() {
                Some(t) => t,
                None => { fail!(); }
            };

            assert_eq!(t, token);

            Ok(())
        }
    }
    */

    //////////////////////////////////////////////////////////////////////////////

    fn test_value<
        'a,
        T: Serializable<'a, Iter>,
        Iter: Iterator<Token<'a>>
    >(value: &'a T, tokens: Vec<Token<'a>>) {
        let mut iter = value.serialize();
        for token in tokens.move_iter() {
            let t = iter.next();
            assert_eq!(t, Some(token));
        }

        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_tokens_null() {
        test_value(&(), vec!(Null));
    }

    #[test]
    fn test_tokens_bool() {
        test_value(&true, vec!(Bool(true)));
        test_value(&false, vec!(Bool(false)));
    }

    #[test]
    fn test_tokens_int() {
        test_value(&5, vec!(Int(5)));
    }

    #[test]
    fn test_tokens_str() {
        test_value(&"abc", vec!(Str("abc")));
    }

    #[test]
    fn test_tokens_string() {
        test_value(&"abc".to_string(), vec!(Str("abc")));
    }

    #[test]
    fn test_tokens_option() {
        test_value(&None::<int>, vec!(Option(false)));
        test_value(&Some(5), vec!(Option(true), Int(5)));
    }

    /*
    #[test]
    fn test_tokens_tuple() {
        let tokens = vec!(
            TupleStart(2),
                Int(5),
                Str("a"),
            End,
        );

        let mut serializer = AssertSerializer::new(tokens.move_iter());
        (5, "a").serialize(&mut serializer).unwrap();
        assert_eq!(serializer.iter.next(), None);
    }

    #[test]
    fn test_tokens_tuple_compound() {
        let tokens = vec!(
            TupleStart(3),
                Null,
                Null,

                TupleStart(2),
                    Int(5),
                    Str("a"),
                End,
            End,
        );

        let mut serializer = AssertSerializer::new(tokens.move_iter());
        ((), (), (5, "a")).serialize(&mut serializer).unwrap();
        assert_eq!(serializer.iter.next(), None);
    }
    */

    #[test]
    fn test_tokens_struct() {
        test_value(
            &Outer { inner: vec!() },
            vec!(
                StructStart("Outer", 1),
                    Str("inner"),
                    SeqStart(0),
                    End,
                End,
            )
        );

        let mut map = HashMap::new();
        map.insert("abc".to_string(), Some('c'));

        test_value(
            &Outer {
                inner: vec!(
                    Inner {
                        a: (),
                        b: 5,
                        c: map,
                    },
                )
            },
            vec!(
                StructStart("Outer", 1),
                    Str("inner"),
                    SeqStart(1),
                        StructStart("Inner", 3),
                            Str("a"),
                            Null,

                            Str("b"),
                            Uint(5),

                            Str("c"),
                            MapStart(1),
                                Str("abc"),

                                Option(true),
                                Char('c'),
                            End,
                        End,
                    End,
                End,
            )
        );
    }

    #[test]
    fn test_tokens_enum() {
        test_value(&Dog, vec!(EnumStart("Animal", "Dog", 0), End));
        test_value(
            &Frog("Henry".to_string(), 349),
            vec!(
                EnumStart("Animal", "Frog", 2),
                Str("Henry"),
                Int(349),
                End
            )
        );
    }

    #[test]
    fn test_tokens_vec() {
        let v: Vec<int> = vec!();
        test_value(&v, vec!(SeqStart(0), End));

        test_value(
            &vec!(1, 2, 3),
            vec!(SeqStart(3), Int(1), Int(2), Int(3), End)
        );

        test_value(
            &vec!(vec!(1), vec!(2, 3), vec!(4, 5, 6)),
            vec!(
                SeqStart(3),
                    SeqStart(1),
                        Int(1),
                    End,

                    SeqStart(2),
                        Int(2),
                        Int(3),
                    End,

                    SeqStart(3),
                        Int(4),
                        Int(5),
                        Int(6),
                    End,
                End,
            )
        );
    }

    #[test]
    fn test_tokens_treemap() {
        let v: TreeMap<int, int> = TreeMap::new();
        test_value(&v, vec!(MapStart(0), End));

        test_value(
            &treemap!("a" => 1, "b" => 2, "c" => 3),
            vec!(
                MapStart(3),
                    Str("a"),
                    Int(1),

                    Str("b"),
                    Int(2),

                    Str("c"),
                    Int(3),
                End
            )
        );

        test_value(
            &treemap!(
                "a" => treemap!(),
                "b" => treemap!("a" => 1),
                "c" => treemap!("a" => 2, "b" => 3)
            ),
            vec!(
                MapStart(3),
                    Str("a"),
                    MapStart(0),
                    End,

                    Str("b"),
                    MapStart(1),
                        Str("a"),
                        Int(1),
                    End,

                    Str("c"),
                    MapStart(2),
                        Str("a"),
                        Int(2),

                        Str("b"),
                        Int(3),
                    End,
                End
            )
        );
    }
}
