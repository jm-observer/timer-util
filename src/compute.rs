// use crate::{Operator, Seconds};
//
// struct TimeUnit<T: Operator> {
//     max: T::ValTy,
//     current: T::ValTy,
//     conf: T,
// }
//
// pub trait Computer {
//     const MIN: Self::ValTy;
//     // const MAX: Self::ValTy;
//     type ValTy;
// }
//
// impl<T: Operator> TimeUnit<T> {
//     pub fn is_match(&self) -> bool {
//         self.conf.contain(self.current)
//     }
// }
//
// impl Computer for TimeUnit<Seconds> {
//     const MIN: Self::ValTy = <Seconds as Operator>::MIN;
//     // const MAX: Self::ValTy = <Seconds as Operator>::MAX;
//     type ValTy = <Seconds as Operator>::ValTy;
// }
