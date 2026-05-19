use crate::conf::TimerConf;
use chrono::NaiveDateTime;

/// An iterator that yields scheduled `NaiveDateTime` values from a `TimerConf`.
///
/// Created by [`TimerConf::iter_from()`] or [`TimerConf::iter_range()`].
pub struct TimerIter<'a> {
    conf: &'a TimerConf,
    current: NaiveDateTime,
    end: Option<NaiveDateTime>,
}

impl<'a> TimerIter<'a> {
    pub(crate) fn new(
        conf: &'a TimerConf,
        start: NaiveDateTime,
        end: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            conf,
            current: start,
            end,
        }
    }
}

impl<'a> Iterator for TimerIter<'a> {
    type Item = NaiveDateTime;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.conf.next_with_time(self.current);
        if self.end.is_some_and(|end| next > end) {
            return None;
        }
        self.current = next;
        Some(next)
    }
}
