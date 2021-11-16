use crate::errors::ErrorCode::{self, *};
use anchor_lang::prelude::*;
use fehler::{throw, throws};
use std::cmp::Ordering;
use std::convert::TryInto;

pub fn to_u128(val: u64) -> Result<u128, ErrorCode> {
    val.try_into().map_err(|_| ConversionFailure)
}

pub fn to_u64(val: u128) -> Result<u64, ErrorCode> {
    val.try_into().map_err(|_| ConversionFailure)
}

pub trait PubkeyPairExt: Sized {
    fn sort<T>(&self, v1: T, v2: T) -> Result<(T, T), ErrorCode>;
    fn sort_self(self) -> Result<Self, ErrorCode>;
}

impl PubkeyPairExt for (Pubkey, Pubkey) {
    #[throws(ErrorCode)]
    fn sort<T>(&self, v1: T, v2: T) -> (T, T) {
        match self.0.key().cmp(&self.1.key()) {
            Ordering::Less => (v1, v2),
            Ordering::Equal => throw!(SameToken),
            Ordering::Greater => (v2, v1),
        }
    }

    #[throws(ErrorCode)]
    fn sort_self(self) -> Self {
        match self.0.key().cmp(&self.1.key()) {
            Ordering::Less => (self.0, self.1),
            Ordering::Equal => throw!(SameToken),
            Ordering::Greater => (self.1, self.0),
        }
    }
}

impl<'a> PubkeyPairExt for (&'a Pubkey, &'a Pubkey) {
    #[throws(ErrorCode)]
    fn sort<T>(&self, v1: T, v2: T) -> (T, T) {
        match self.0.key().cmp(&self.1.key()) {
            Ordering::Less => (v1, v2),
            Ordering::Equal => throw!(SameToken),
            Ordering::Greater => (v2, v1),
        }
    }

    #[throws(ErrorCode)]
    fn sort_self(self) -> Self {
        match self.0.key().cmp(&self.1.key()) {
            Ordering::Less => (self.0, self.1),
            Ordering::Equal => throw!(SameToken),
            Ordering::Greater => (self.1, self.0),
        }
    }
}

impl<'a, 'info, A> PubkeyPairExt for (&'a Account<'info, A>, &'a Account<'info, A>)
where
    A: AccountSerialize + AccountDeserialize + Owner + Clone,
{
    #[throws(ErrorCode)]
    fn sort<T>(&self, v1: T, v2: T) -> (T, T) {
        match self.0.key().cmp(&self.1.key()) {
            Ordering::Less => (v1, v2),
            Ordering::Equal => throw!(SameToken),
            Ordering::Greater => (v2, v1),
        }
    }

    #[throws(ErrorCode)]
    fn sort_self(self) -> Self {
        match self.0.key().cmp(&self.1.key()) {
            Ordering::Less => (self.0, self.1),
            Ordering::Equal => throw!(SameToken),
            Ordering::Greater => (self.1, self.0),
        }
    }
}

impl<'a, 'info, A> PubkeyPairExt for (&'a mut Account<'info, A>, &'a mut Account<'info, A>)
where
    A: AccountSerialize + AccountDeserialize + Owner + Clone,
{
    #[throws(ErrorCode)]
    fn sort<T>(&self, v1: T, v2: T) -> (T, T) {
        match self.0.key().cmp(&self.1.key()) {
            Ordering::Less => (v1, v2),
            Ordering::Equal => throw!(SameToken),
            Ordering::Greater => (v2, v1),
        }
    }

    #[throws(ErrorCode)]
    fn sort_self(self) -> Self {
        match self.0.key().cmp(&self.1.key()) {
            Ordering::Less => (self.0, self.1),
            Ordering::Equal => throw!(SameToken),
            Ordering::Greater => (self.1, self.0),
        }
    }
}

pub trait TupleExt {
    type Elem;

    fn contains(&self, e: &Self::Elem) -> bool;
}

impl<A: PartialEq> TupleExt for (A, A) {
    type Elem = A;

    fn contains(&self, e: &Self::Elem) -> bool {
        &self.0 == e || &self.1 == e
    }
}
