use std::{
    fmt::{self, Debug},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    CellId, ReadRef, TaskId,
    manager::{KeyedReadDependency, with_turbo_tasks},
    vc::{VcReadTarget, VcValueType},
};

/// The panic message when a [`LazyReadRef`] or a pending
/// [`Vc::lazy_reader`](crate::Vc::lazy_reader) future is used in a different task execution than
/// the one that created it. Only checked in debug builds.
pub(crate) const LAZY_READ_CROSS_EXECUTION: &str =
    "A `LazyReadRef` (or a pending `lazy_reader()` future) was used outside of the task execution \
     that created it. It must not be moved to other threads or tasks.";

/// The result of [`Vc::lazy_reader`](crate::Vc::lazy_reader): a cell read that can be canceled.
///
/// The value is only accessible via [`LazyReadRef::get`]. When the `LazyReadRef` is dropped
/// without [`get`](LazyReadRef::get) ever being called, the read is canceled and the task no longer
/// depends on the cell through it. Otherwise it behaves like a normal read (`.await`).
///
/// This allows reading a cell that is rarely needed ahead of time, e.g. to pass it into
/// synchronous code that decides whether it needs the value, without depending on it when it
/// isn't needed.
///
/// It **must** be dropped within the task execution that read it. Otherwise the task fails with a
/// panic error and keeps the dependency.
#[must_use]
pub struct LazyReadRef<T>
where
    T: VcValueType,
{
    read_ref: ReadRef<T>,
    /// `None` if the read wasn't tracked (e.g. a read outside of a task execution), or isn't
    /// removable (the read continued in another task execution).
    dependency: Option<(TaskId, CellId, KeyedReadDependency)>,
    accessed: AtomicBool,
}

impl<T> LazyReadRef<T>
where
    T: VcValueType,
{
    /// `dependency` must only be `Some` within the task execution that created it.
    pub(crate) fn new(
        read_ref: ReadRef<T>,
        dependency: Option<(TaskId, CellId, KeyedReadDependency)>,
    ) -> Self {
        if let Some((_, _, dependency)) = &dependency {
            dependency.register_alive_ref();
        }
        Self {
            read_ref,
            dependency,
            accessed: AtomicBool::new(false),
        }
    }

    /// Accesses the value. Once this is called, the task depends on the value like with a normal
    /// read, even when it's only called conditionally.
    pub fn get(&self) -> &VcReadTarget<T> {
        #[cfg(debug_assertions)]
        if let Some((_, _, dependency)) = &self.dependency {
            assert!(
                dependency.is_current_execution(),
                "{LAZY_READ_CROSS_EXECUTION}"
            );
        }
        // Only the creating execution reads the flag (when dropping the ref), so relaxed is
        // enough.
        self.accessed.store(true, Ordering::Relaxed);
        &self.read_ref
    }
}

impl<T> Drop for LazyReadRef<T>
where
    T: VcValueType,
{
    fn drop(&mut self) {
        let Some((task, index, dependency)) = self.dependency.take() else {
            return;
        };
        if !dependency.is_current_execution() {
            // Keep the conservative dependency on the whole cell. The execution fails as it
            // still counts this `LazyReadRef` as alive. Don't panic while unwinding, as that
            // would abort.
            #[cfg(debug_assertions)]
            if !std::thread::panicking() {
                panic!("{LAZY_READ_CROSS_EXECUTION}");
            }
            return;
        }
        dependency.release_alive_ref();
        if *self.accessed.get_mut() {
            // Keep the dependency. It's converted to a normal dependency when the execution
            // completes.
            return;
        }
        // Converting the dependency to no hashed dependencies at all removes it.
        with_turbo_tasks(|tt| {
            tt.convert_keyed_cell_dependency(
                task,
                index,
                dependency.reader,
                dependency.id,
                &mut std::iter::empty(),
            )
        });
    }
}

impl<T> Debug for LazyReadRef<T>
where
    T: VcValueType,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Must not access the value, as that would count as an access.
        f.debug_struct("LazyReadRef")
            .field("accessed", &self.accessed.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}
