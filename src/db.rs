use std::pin::Pin;

use async_trait::async_trait;
use futures::{future::join_all, Future};

use crate::{
    deinflect::Reasons,
    terms_bank::Term,
    translator::{get_grouped_terms, DictEntries},
    Dict, YomiDictError,
};

type StepFuture<'a> = dyn Future<Output = Result<usize, YomiDictError>> + 'a;
pub struct DictInsertionSteps<'a> {
    pub total_count: usize,
    pub steps: Vec<Pin<Box<StepFuture<'a>>>>,
}

#[async_trait(?Send)]
pub trait DBImpl {
    async fn add_dict_stepwise(&self, dict: Dict) -> Result<DictInsertionSteps<'_>, YomiDictError>;
    async fn get_raw_matches(
        &self,
        term_list: impl IntoIterator<Item = &str>,
    ) -> Result<Vec<Term>, YomiDictError>;
}

#[async_trait(?Send)]
pub trait DB {
    async fn add_dict(&self, dict: Dict) -> Result<(), YomiDictError>;
    async fn find_terms(
        &self,
        text: &str,
        reasons: &Reasons,
    ) -> Result<Vec<DictEntries>, YomiDictError>;
    async fn add_dict_stepwise(&self, dict: Dict) -> Result<DictInsertionSteps<'_>, YomiDictError>;
}

#[async_trait(?Send)]
impl<T: DBImpl> DB for T {
    async fn add_dict(&self, dict: Dict) -> Result<(), YomiDictError> {
        let steps = self.add_dict_stepwise(dict).await?;
        let should_total = steps.total_count;

        let total = join_all(steps.steps)
            .await
            .into_iter()
            .sum::<Result<usize, _>>()?;

        debug_assert_eq!(should_total, total);

        Ok(())
    }

    async fn add_dict_stepwise(&self, dict: Dict) -> Result<DictInsertionSteps<'_>, YomiDictError> {
        self.add_dict_stepwise(dict).await
    }

    async fn find_terms(
        &self,
        text: &str,
        reasons: &Reasons,
    ) -> Result<Vec<DictEntries>, YomiDictError> {
        get_grouped_terms(text, reasons, self).await
    }
}
