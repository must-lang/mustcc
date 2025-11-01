use cranelift_module::FuncId;

pub struct Env {}
impl Env {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn add_func(&self, id: crate::common::NodeID, func_id: FuncId) {
        todo!()
    }
}
