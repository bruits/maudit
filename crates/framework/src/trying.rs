// ANCHOR: All
use std::any::{Any, TypeId};
use std::marker::PhantomData;

use rustc_hash::FxHashMap;

use crate::page::RenderResult;
pub struct FunctionSystem<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

pub trait System {
    fn run(&mut self, resources: &mut FxHashMap<TypeId, Box<dyn Any>>) -> RenderResult;
}

impl<F: FnMut() -> RenderResult> System for FunctionSystem<(), F> {
    fn run(&mut self, resources: &mut FxHashMap<TypeId, Box<dyn Any>>) -> RenderResult {
        (self.f)()
    }
}

impl<F: FnMut(T1) -> RenderResult, T1: 'static> System for FunctionSystem<(T1,), F> {
    fn run(&mut self, resources: &mut FxHashMap<TypeId, Box<dyn Any>>) -> RenderResult {
        let _0 = *resources
            .remove(&TypeId::of::<T1>())
            .unwrap()
            .downcast::<T1>()
            .unwrap();

        (self.f)(_0)
    }
}

impl<F: FnMut(T1, T2) -> RenderResult, T1: 'static, T2: 'static> System
    for FunctionSystem<(T1, T2), F>
{
    fn run(&mut self, resources: &mut FxHashMap<TypeId, Box<dyn Any>>) -> RenderResult {
        let _0 = *resources
            .remove(&TypeId::of::<T1>())
            .unwrap()
            .downcast::<T1>()
            .unwrap();
        let _1 = *resources
            .remove(&TypeId::of::<T2>())
            .unwrap()
            .downcast::<T2>()
            .unwrap();

        (self.f)(_0, _1)
    }
}

pub trait IntoSystem<Input> {
    type System: System;

    fn into_system(self) -> Self::System;
}

impl<F: FnMut() -> RenderResult> IntoSystem<()> for F {
    type System = FunctionSystem<(), Self>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            f: self,
            marker: Default::default(),
        }
    }
}

impl<F: FnMut(T1) -> RenderResult, T1: 'static> IntoSystem<(T1,)> for F {
    type System = FunctionSystem<(T1,), Self>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            f: self,
            marker: Default::default(),
        }
    }
}

impl<F: FnMut(T1, T2) -> RenderResult, T1: 'static, T2: 'static> IntoSystem<(T1, T2)> for F {
    type System = FunctionSystem<(T1, T2), Self>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            f: self,
            marker: Default::default(),
        }
    }
}

type StoredSystem = Box<dyn System>;

pub struct Scheduler {
    pub system: StoredSystem,
    pub resources: FxHashMap<TypeId, Box<dyn Any>>,
}

struct Something;

impl Something {
    fn do_something(&self) -> RenderResult {
        println!("Something was done");

        RenderResult::Text("Something was done".to_string())
    }
}

impl Scheduler {
    pub fn run(&mut self) -> RenderResult {
        self.system.run(&mut self.resources)
    }

    pub fn add_resource<R: 'static>(&mut self, res: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(res));
    }
}
// ANCHOR_END: All

pub fn do_try() {
    let mut scheduler = Scheduler {
        system: Box::new(Something::do_something.into_system()),
        resources: FxHashMap::default(),
    };

    scheduler.add_resource(&Something);
    scheduler.add_resource(12i32);

    scheduler.run();
}

fn foo(int: i32) {
    println!("int! {int}");
}
