use std::time::Duration;

use rodio::source::SeekError;
use rodio::{ChannelCount, SampleRate, Source};

pub(super) struct DoneCallback<I, F>
where
    F: FnOnce(),
{
    input: I,
    callback: Option<F>,
}

impl<I, F> DoneCallback<I, F>
where
    F: FnOnce(),
{
    pub fn new(input: I, callback: F) -> Self {
        Self {
            input,
            callback: Some(callback),
        }
    }
}

impl<I, F> Iterator for DoneCallback<I, F>
where
    I: Source,
    F: FnOnce(),
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        let next = self.input.next();

        if next.is_none()
            && let Some(callback) = self.callback.take()
        {
            callback();
        }

        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I, F> Source for DoneCallback<I, F>
where
    I: Source,
    F: FnOnce(),
{
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    #[inline]
    fn channels(&self) -> ChannelCount {
        self.input.channels()
    }

    #[inline]
    fn sample_rate(&self) -> SampleRate {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.input.try_seek(pos)
    }
}
