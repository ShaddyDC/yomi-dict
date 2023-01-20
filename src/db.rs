use std::pin::Pin;

use async_trait::async_trait;
use futures::Future;

use crate::{terms_bank::Term, Dict, YomiDictError};

type StepFuture<'a> = dyn Future<Output = Result<usize, YomiDictError>> + 'a;
pub struct DictInsertionSteps<'a> {
    pub total_count: usize,
    pub steps: Vec<Pin<Box<StepFuture<'a>>>>,
}

#[async_trait(?Send)]
pub trait DB {
    async fn add_dict(&self, dict: Dict) -> Result<(), YomiDictError>;
    async fn add_dict_stepwise(&self, dict: Dict) -> Result<DictInsertionSteps<'_>, YomiDictError>;
    async fn get_terms(
        &self,
        term_list: impl IntoIterator<Item = &str>,
    ) -> Result<Vec<Term>, YomiDictError>;
}
