//! `RUID`: a rolled index wearing a type of its own, and a record of where it came from.
//!
//! The type used to promise more than it could keep. It was described as a runtime-unique
//! id, and `RUID::from(5)` made one out of nothing, while `a + b` made another out of two
//! more. Both produced a value indistinguishable from a rolled one, so the guarantee held
//! only for as long as nobody used the rest of the surface.
//!
//! What closes that is saying where a value came from, in its type. `RUID<Rolled>` is one
//! the counter handed out, and no safe path produces one except asking the counter.
//! `RUID<Derived>` is anything else: built from an integer, parsed, or computed. The two
//! compare and print alike, and a rolled id converts into a derived one freely because it
//! genuinely is one, but nothing goes the other way.

use crate::{rolling_idx, Idx};
use core::cmp::Ordering;
use core::fmt;
use core::marker::PhantomData;
use core::str::FromStr;

#[cfg(not(feature = "const"))]
use core::ops::Deref;

#[cfg(feature = "allow_arithmetics")]
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Rolled {}
    impl Sealed for super::Derived {}
}

/// Where a `RUID`'s value came from.
///
/// Sealed: the two answers below are the only ones, and a third would be a claim this
/// crate has no way to check.
pub trait Provenance: sealed::Sealed {
    /// Whether values carrying this provenance came from the rolling index.
    const ROLLED: bool;
}

/// The value came from the rolling index, so no other `RUID<Rolled>` in this run holds it.
///
/// The only way to obtain one is [`RUID::new`].
#[derive(Debug)]
pub enum Rolled {}

/// The value came from somewhere else: an integer, a parse, or arithmetic.
///
/// It may collide with anything. That is not a defect in it, it is why it is not called
/// `Rolled`.
#[derive(Debug)]
pub enum Derived {}

impl Provenance for Rolled {
    const ROLLED: bool = true;
}

impl Provenance for Derived {
    const ROLLED: bool = false;
}

/// A rolling unique id.
///
/// The parameter records where the value came from and defaults to [`Rolled`], so a bare
/// `RUID` means the kind carrying the guarantee.
#[cfg(not(feature = "const"))]
pub struct RUID<P: Provenance = Rolled> {
    value: Idx,
    provenance: PhantomData<P>,
}

/// A rolling unique id, taking its value on first read.
///
/// The `const` feature makes [`RUID::new`] a `const fn`, so a `RUID` can be a `static` or
/// an associated constant. Taking an index needs a shared counter and cannot happen at
/// compile time, so the value starts unassigned and the first read takes one.
#[cfg(feature = "const")]
pub struct RUID<P: Provenance = Rolled> {
    value: std::sync::OnceLock<Idx>,
    provenance: PhantomData<P>,
}

// The field is private and there is no `From<Idx> for RUID<Rolled>`, which together are
// what make the parameter mean anything: a `RUID<Rolled>` is reachable only by asking the
// counter. Every constructor below that takes a value produces a `Derived`.

#[cfg(not(feature = "const"))]
impl<P: Provenance> RUID<P> {
    #[inline]
    const fn wrap(value: Idx) -> Self {
        Self { value, provenance: PhantomData }
    }

    /// The underlying index.
    #[inline]
    #[must_use]
    pub const fn get(&self) -> Idx {
        self.value
    }
}

#[cfg(feature = "const")]
impl<P: Provenance> RUID<P> {
    #[inline]
    fn wrap(value: Idx) -> Self {
        let cell = std::sync::OnceLock::new();
        let _ = cell.set(value);
        Self { value: cell, provenance: PhantomData }
    }

    /// The underlying index, taking one on the first call and keeping it after.
    ///
    /// One implementation serves both provenances. A derived id is assigned when it is
    /// made, so the cell is already full and the closure never runs; a rolled one starts
    /// empty and fills here. Two threads reading an unassigned id at once agree on the
    /// answer, because one wins the initialisation and the other sees the winner's value.
    #[inline]
    #[must_use]
    pub fn get(&self) -> Idx {
        *self.value.get_or_init(rolling_idx)
    }
}

impl<P: Provenance> RUID<P> {
    /// Whether this id came from the rolling index.
    ///
    /// The answer is fixed by the type, so this is for generic code holding a `RUID<P>`
    /// that wants to report which kind it has.
    #[inline]
    #[must_use]
    pub const fn is_rolled(&self) -> bool {
        P::ROLLED
    }
}

#[cfg(feature = "const")]
impl RUID<Rolled> {
    /// Constructs an unassigned id, which takes its index the first time it is read.
    ///
    /// This is the `const fn` the feature exists for.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { value: std::sync::OnceLock::new(), provenance: PhantomData }
    }

}

#[cfg(not(feature = "const"))]
impl RUID<Rolled> {
    /// Takes the next index from the rolling counter.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::wrap(rolling_idx())
    }
}

impl RUID<Rolled> {
    /// Forgets the guarantee, keeping the value.
    ///
    /// A rolled id is a perfectly good derived one, so this always succeeds. There is
    /// deliberately no way back: nothing this crate could check would turn a value that
    /// came from elsewhere into one the counter handed out.
    #[inline]
    #[must_use]
    pub fn into_derived(self) -> RUID<Derived> {
        RUID::<Derived>::wrap(self.get())
    }

    /// The same, without giving up the original.
    ///
    /// `into_derived` consumes, which is free when `RUID` is `Copy` and is a real cost
    /// under the `const` feature, where it is not. A readme example caught that by
    /// failing to compile.
    #[inline]
    #[must_use]
    pub fn to_derived(&self) -> RUID<Derived> {
        RUID::<Derived>::wrap(self.get())
    }
}

impl Default for RUID<Rolled> {
    /// The same as `new`, so a `RUID` inside a `#[derive(Default)]` type still gets an id
    /// rather than a zero that collides with every other default.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// There is deliberately no `Default for RUID<Derived>`. A default derived id would be
// zero, which collides with every other default and with the counter's own first value,
// and having two `Default` impls also made the bare `RUID::default()` ambiguous.

// A rolled id is copied rather than cloned, so passing one around neither consumes it nor
// takes a new index. Under `const` the value lives in a `OnceLock`, which cannot be
// copied, and cloning is what remains.
#[cfg(not(feature = "const"))]
impl<P: Provenance> Copy for RUID<P> {}

#[cfg(not(feature = "const"))]
impl<P: Provenance> Clone for RUID<P> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(feature = "const")]
impl<P: Provenance> Clone for RUID<P> {
    /// Clones the id, taking one first if it has not been read yet.
    ///
    /// The clone carries the same id rather than the next one, which is what makes a
    /// `RUID` behave like the value it is rather than like a generator.
    fn clone(&self) -> Self {
        Self::wrap(self.get())
    }
}

#[cfg(not(feature = "const"))]
impl<P: Provenance> Deref for RUID<P> {
    type Target = Idx;

    #[inline]
    fn deref(&self) -> &Idx {
        &self.value
    }
}

#[cfg(not(feature = "const"))]
impl<P: Provenance> core::borrow::Borrow<Idx> for RUID<P> {
    /// So a map keyed by `RUID` can be looked up with a bare index.
    #[inline]
    fn borrow(&self) -> &Idx {
        &self.value
    }
}

#[cfg(not(feature = "const"))]
impl<P: Provenance> AsRef<Idx> for RUID<P> {
    #[inline]
    fn as_ref(&self) -> &Idx {
        &self.value
    }
}

// Comparison ignores provenance, because two ids are the same id when they name the same
// thing. Separating them here would let a map hold both and answer for neither.
impl<P: Provenance, Q: Provenance> PartialEq<RUID<Q>> for RUID<P> {
    #[inline]
    fn eq(&self, other: &RUID<Q>) -> bool {
        self.get() == other.get()
    }
}

impl<P: Provenance> Eq for RUID<P> {}

impl<P: Provenance, Q: Provenance> PartialOrd<RUID<Q>> for RUID<P> {
    #[inline]
    fn partial_cmp(&self, other: &RUID<Q>) -> Option<Ordering> {
        Some(self.get().cmp(&other.get()))
    }
}

impl<P: Provenance> Ord for RUID<P> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<P: Provenance> core::hash::Hash for RUID<P> {
    /// Hashes as the underlying index, so `Hash` agrees with `Eq` across provenance and a
    /// `RUID` keys a map the way the integer would.
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

// Comparing against a bare integer is what `strict` withholds. The flag carries two
// unrelated meanings, and this is the second: what happens when the width runs out, and
// whether an id is opaque. One flag for both is a wart, named rather than papered over.
#[cfg(not(feature = "strict"))]
impl<P: Provenance> PartialEq<Idx> for RUID<P> {
    #[inline]
    fn eq(&self, other: &Idx) -> bool {
        self.get() == *other
    }
}

#[cfg(not(feature = "strict"))]
impl<P: Provenance> PartialOrd<Idx> for RUID<P> {
    #[inline]
    fn partial_cmp(&self, other: &Idx) -> Option<Ordering> {
        self.get().partial_cmp(other)
    }
}

macro_rules! formatting {
    ($($trait:ident),+ $(,)?) => {$(
        impl<P: Provenance> fmt::$trait for RUID<P> {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::$trait::fmt(&self.get(), f)
            }
        }
    )+};
}

// Delegating rather than reimplementing means width, padding and the `#` flag all behave
// the way they do on the integer, instead of being silently dropped.
formatting!(Display, Binary, Octal, LowerHex, UpperHex);

impl<P: Provenance> fmt::Debug for RUID<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RUID").field(&self.get()).finish()
    }
}

impl From<Idx> for RUID<Derived> {
    /// Wraps an index already in hand. This takes no new index, and the result is
    /// `Derived` because nothing here checks that the counter ever produced it.
    #[inline]
    fn from(value: Idx) -> Self {
        Self::wrap(value)
    }
}

impl From<RUID<Rolled>> for RUID<Derived> {
    #[inline]
    fn from(id: RUID<Rolled>) -> Self {
        id.into_derived()
    }
}

impl<P: Provenance> From<RUID<P>> for Idx {
    #[inline]
    fn from(id: RUID<P>) -> Self {
        id.get()
    }
}

impl FromStr for RUID<Derived> {
    type Err = core::num::ParseIntError;

    /// Parses an index. `Derived`, for the same reason `From<Idx>` is.
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Idx>().map(Self::wrap)
    }
}

/// The arithmetic operators, which are all the same shape.
///
/// Every one produces a `RUID<Derived>` whatever it was given, and that is the point
/// rather than a limitation. Adding two ids gives a number the counter never handed out,
/// and which may well collide with one it did, so a result claiming to be `Rolled` would
/// be claiming something false.
///
/// Writing them out one at a time was 250 lines in which no reader could have spotted a
/// transposed operator, so they are generated from the list instead.
#[cfg(feature = "allow_arithmetics")]
macro_rules! arithmetic {
    ($( $op:ident $fn_name:ident $assign:ident $assign_fn:ident $sym:tt ),+ $(,)?) => {$(
        impl<P: Provenance, Q: Provenance> $op<RUID<Q>> for RUID<P> {
            type Output = RUID<Derived>;
            #[inline]
            fn $fn_name(self, other: RUID<Q>) -> RUID<Derived> {
                RUID::<Derived>::wrap(self.get() $sym other.get())
            }
        }

        impl<P: Provenance> $op<Idx> for RUID<P> {
            type Output = RUID<Derived>;
            #[inline]
            fn $fn_name(self, other: Idx) -> RUID<Derived> {
                RUID::<Derived>::wrap(self.get() $sym other)
            }
        }

        // The same two by reference. Under `const` a `RUID` holds a `OnceLock` and cannot
        // be `Copy`, so every by-value operator consumes it: `a + b` leaves neither
        // usable and `a + 1` afterwards does not compile. Without these, `const` and
        // `allow_arithmetics` together are close to unusable, and they are one of the
        // crate's own declared feature combinations.
        impl<P: Provenance, Q: Provenance> $op<&RUID<Q>> for &RUID<P> {
            type Output = RUID<Derived>;
            #[inline]
            fn $fn_name(self, other: &RUID<Q>) -> RUID<Derived> {
                RUID::<Derived>::wrap(self.get() $sym other.get())
            }
        }

        impl<P: Provenance> $op<Idx> for &RUID<P> {
            type Output = RUID<Derived>;
            #[inline]
            fn $fn_name(self, other: Idx) -> RUID<Derived> {
                RUID::<Derived>::wrap(self.get() $sym other)
            }
        }

        // Assigning forms exist only for `Derived`. Applying one to a rolled id would
        // change it in place into a value the counter never gave, which is the hole this
        // whole arrangement closes, so the type refuses instead.
        impl<Q: Provenance> $assign<RUID<Q>> for RUID<Derived> {
            #[inline]
            fn $assign_fn(&mut self, other: RUID<Q>) {
                *self = Self::wrap(self.get() $sym other.get());
            }
        }

        impl $assign<Idx> for RUID<Derived> {
            #[inline]
            fn $assign_fn(&mut self, other: Idx) {
                *self = Self::wrap(self.get() $sym other);
            }
        }
    )+};
}

#[cfg(feature = "allow_arithmetics")]
arithmetic!(
    Add add AddAssign add_assign +,
    Sub sub SubAssign sub_assign -,
    Mul mul MulAssign mul_assign *,
    Div div DivAssign div_assign /,
    Rem rem RemAssign rem_assign %,
);
