use crate::configuration::LldapConfig;

#[derive(Clone, Debug)]
pub struct LldapClient {
    pub config: LldapConfig,
}
