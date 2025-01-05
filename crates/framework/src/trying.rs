// ANCHOR: All
use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::page::RenderResult;
use rustc_hash::FxHashMap;

pub struct FunctionSystem<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

pub trait System {
    fn run(&mut self, resources: &mut FxHashMap<TypeId, RefCell<Box<dyn Any>>>) -> RenderResult;
}

pub trait IntoSystem<Input> {
    type System: System;

    fn into_system(self) -> Self::System;
}

pub type StoredSystem = Box<dyn System>;

pub struct Scheduler {
    pub system: StoredSystem,
    pub resources: FxHashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl Scheduler {
    pub fn run(&mut self) -> RenderResult {
        self.system.run(&mut self.resources)
    }

    pub fn add_system<I, S: System + 'static>(&mut self, system: impl IntoSystem<I, System = S>) {
        self.system = Box::new(system.into_system());
    }

    pub fn add_resource<R: 'static>(&mut self, res: R) {
        self.resources
            .insert(TypeId::of::<R>(), RefCell::new(Box::new(res)));
    }
}

pub trait SystemParam {
    type Item<'new>;

    fn retrieve<'r>(resources: &'r FxHashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r>;
}

pub struct Res<'a, T: 'static> {
    pub value: Ref<'a, Box<dyn Any>>,
    _marker: PhantomData<&'a T>,
}

impl<T: 'static> Deref for Res<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value.downcast_ref().unwrap()
    }
}

impl<'res, T: 'static> SystemParam for Res<'res, T> {
    type Item<'new> = Res<'new, T>;

    fn retrieve<'r>(resources: &'r FxHashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r> {
        Res {
            value: resources.get(&TypeId::of::<T>()).unwrap().borrow(),
            _marker: PhantomData,
        }
    }
}

pub struct ResMut<'a, T: 'static> {
    value: RefMut<'a, Box<dyn Any>>,
    _marker: PhantomData<&'a mut T>,
}

impl<T: 'static> Deref for ResMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value.downcast_ref().unwrap()
    }
}

impl<T: 'static> DerefMut for ResMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value.downcast_mut().unwrap()
    }
}

impl<'res, T: 'static> SystemParam for ResMut<'res, T> {
    type Item<'new> = ResMut<'new, T>;

    fn retrieve<'r>(resources: &'r FxHashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r> {
        ResMut {
            value: resources.get(&TypeId::of::<T>()).unwrap().borrow_mut(),
            _marker: PhantomData,
        }
    }
}

impl<F, T1: SystemParam> System for FunctionSystem<(T1,), F>
where
    for<'a, 'b> &'a mut F:
        FnMut(T1) -> RenderResult + FnMut(<T1 as SystemParam>::Item<'b>) -> RenderResult,
{
    fn run(&mut self, resources: &mut FxHashMap<TypeId, RefCell<Box<dyn Any>>>) -> RenderResult {
        // necessary to tell rust exactly which impl to call; it gets a bit confused otherwise
        fn call_inner<T1>(mut f: impl FnMut(T1) -> RenderResult, _0: T1) -> RenderResult {
            f(_0)
        }

        let _0 = T1::retrieve(resources);

        call_inner(&mut self.f, _0)
    }
}

impl<F, T1: SystemParam, T2: SystemParam> System for FunctionSystem<(T1, T2), F>
where
    for<'a, 'b> &'a mut F: FnMut(T1, T2) -> RenderResult
        + FnMut(<T1 as SystemParam>::Item<'b>, <T2 as SystemParam>::Item<'b>) -> RenderResult,
{
    fn run(&mut self, resources: &mut FxHashMap<TypeId, RefCell<Box<dyn Any>>>) -> RenderResult {
        // necessary to tell rust exactly which impl to call; it gets a bit confused otherwise
        fn call_inner<T1, T2>(
            mut f: impl FnMut(T1, T2) -> RenderResult,
            _0: T1,
            _1: T2,
        ) -> RenderResult {
            f(_0, _1)
        }

        let _0 = T1::retrieve(resources);
        let _1 = T2::retrieve(resources);

        call_inner(&mut self.f, _0, _1)
    }
}

impl<F: FnMut(T1) -> RenderResult, T1: SystemParam> IntoSystem<(T1,)> for F
where
    for<'a, 'b> &'a mut F:
        FnMut(T1) -> RenderResult + FnMut(<T1 as SystemParam>::Item<'b>) -> RenderResult,
{
    type System = FunctionSystem<(T1,), Self>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            f: self,
            marker: Default::default(),
        }
    }
}

impl<F: FnMut(T1, T2) -> RenderResult, T1: SystemParam, T2: SystemParam> IntoSystem<(T1, T2)> for F
where
    for<'a, 'b> &'a mut F: FnMut(T1, T2) -> RenderResult
        + FnMut(<T1 as SystemParam>::Item<'b>, <T2 as SystemParam>::Item<'b>) -> RenderResult,
{
    type System = FunctionSystem<(T1, T2), Self>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            f: self,
            marker: Default::default(),
        }
    }
}
// ANCHOR_END: All
