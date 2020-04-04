//! Types for binning time into Day/milli-offset, Week/second-offset, Month/seconds-offset,
//! Year/minutes-offset bins, `BinnedTime`.
//!
//!
//! Construct a number of milliseconds as a number of days and milliseconds.
//! ```
//!  use geomesa_z3::binned_time::{BinnedTime, TimePeriod, TimeUnits};
//!
//!  let bin = BinnedTime::from_millis(TimePeriod::Day, 90_000_000);
//!
//!  assert_eq!(bin.bin, 1);
//!  assert_eq!(bin.offset, TimeUnits::Milliseconds(3_600_000));
//! ```
//! Construct a number of milliseconds as a number of weeks and seconds.
//! ```
//!  use geomesa_z3::binned_time::{BinnedTime, TimePeriod, TimeUnits};
//!
//!  let bin = BinnedTime::from_millis(TimePeriod::Week, 1_512_000_000);
//!
//!  assert_eq!(bin.bin, 2);
//!  assert_eq!(bin.offset, TimeUnits::Seconds(302_400));
//! ```
//!
//! Construct a number of milliseconds as a number of months and seconds.
//! ```
//! use geomesa_z3::binned_time::{BinnedTime, TimePeriod, TimeUnits};
//!
//! let bin = BinnedTime::from_millis(TimePeriod::Month, 17_366_400_000);
//!
//! assert_eq!(bin.bin, 6);
//! assert_eq!(bin.offset, TimeUnits::Seconds(1_296_000));
//! ```
//! Construct a number of milliseconds as a number of years and minutes.
//! ```
//! use geomesa_z3::binned_time::{BinnedTime, TimePeriod, TimeUnits};
//!
//! let bin = BinnedTime::from_millis(TimePeriod::Year, 1_586_260_800_000);
//!
//! assert_eq!(bin.bin, 50);
//! assert_eq!(bin.offset, TimeUnits::Minutes(22_693_680));
//! ```

use time::{Duration, OffsetDateTime};

trait BinnedTimeToDate = Fn(BinnedTime) -> OffsetDateTime;
trait TimeToBinnedTime = Fn(i64) -> BinnedTime;
trait DateToBinnedTime = Fn(OffsetDateTime) -> BinnedTime;
trait TimeToBin = Fn(i64) -> i16;
trait DateToBin = Fn(OffsetDateTime) -> i16;

const DAYS_IN_MONTH: i64 = 31;

const WEEKS_IN_YEAR: i64 = 52;

const EPOCH: OffsetDateTime = OffsetDateTime::unix_epoch();

/// The number of `TimePeriod` bins in the `BinnedTime`.
pub type BinIndex = i64;

/// Representation of a datetime as a number of `TimePeriod` bins and an offset from the last bin.
pub struct BinnedTime {
    /// Number of `TimePeriods` since unix epoch.
    pub bin: BinIndex,
    /// Number of milliseconds, seconds, minutes (depending on `TimePeriod`) since unix epoch.
    pub offset: TimeUnits,
}

impl BinnedTime {
    /// Returns a `BinnedTime` struct representing the milliseconds since Unix Epoch, millis.
    #[must_use]
    pub fn from_millis(period: TimePeriod, millis: i64) -> BinnedTime {
        match period {
            TimePeriod::Day => Self::millis_to_day_and_millis(millis),
            TimePeriod::Week => Self::millis_to_week_and_seconds(millis),
            TimePeriod::Month => Self::millis_to_month_and_seconds(millis),
            TimePeriod::Year => Self::millis_to_year_and_minutes(millis),
        }
    }

    /// Returns a `BinnedTime` struct representing the `TimePeriods` since Unix Epoch.
    #[must_use]
    pub fn from_datetime(period: TimePeriod, datetime: OffsetDateTime) -> BinnedTime {
        match period {
            TimePeriod::Day => Self::millis_to_day_and_millis_(datetime - EPOCH),
            TimePeriod::Week => Self::millis_to_week_and_seconds_(datetime - EPOCH),
            TimePeriod::Month => Self::millis_to_month_and_seconds_(datetime - EPOCH),
            TimePeriod::Year => Self::millis_to_year_and_minutes_(datetime - EPOCH),
        }
    }

    /// Number of `TimePeriod` bins that the time in millis represents.
    #[must_use]
    pub fn millis_to_bin_index(period: TimePeriod, millis: i64) -> BinIndex {
        match period {
            TimePeriod::Day => Duration::milliseconds(millis).whole_days(),
            TimePeriod::Week => Duration::milliseconds(millis).whole_weeks(),
            TimePeriod::Month => Duration::milliseconds(millis).whole_days() / DAYS_IN_MONTH as i64,
            TimePeriod::Year => Duration::milliseconds(millis).whole_weeks() / WEEKS_IN_YEAR as i64,
        }
    }

    /// Number of whole `TimePeriod` bins in the datetime.
    #[must_use]
    pub fn datetime_to_bin_index(period: TimePeriod, datetime: OffsetDateTime) -> BinIndex {
        match period {
            TimePeriod::Day => (datetime - EPOCH).whole_days(),
            TimePeriod::Week => (datetime - EPOCH).whole_weeks(),
            TimePeriod::Month => (datetime - EPOCH).whole_days() / DAYS_IN_MONTH as i64,
            TimePeriod::Year => (datetime - EPOCH).whole_weeks() / WEEKS_IN_YEAR as i64,
        }
    }

    /// Return a function that filters datetimes to be representable by a BinnedTime.
    pub fn bounds_to_indexable_dates(
        period: TimePeriod,
    ) -> impl Fn((Option<OffsetDateTime>, Option<OffsetDateTime>)) -> (OffsetDateTime, OffsetDateTime)
    {
        let max_date = Self::max_date(period) - Duration::milliseconds(1);

        move |(low, high)| {
            let low = match low {
                None => EPOCH,
                Some(dt) if dt < EPOCH => EPOCH,
                Some(dt) if dt > max_date => max_date,
                Some(dt) => dt,
            };

            let high = match high {
                None => Self::max_date(period),
                Some(dt) if dt < EPOCH => EPOCH,
                Some(dt) if dt > max_date => max_date,
                Some(dt) => dt,
            };
            (low, high)
        }
    }

    /// The maximum date representable by the BinnedTime of a particular TimePeriod.
    pub fn max_date(period: TimePeriod) -> OffsetDateTime {
        match period {
            TimePeriod::Day | TimePeriod::Week | TimePeriod::Month | TimePeriod::Year => {
                EPOCH + Duration::max_value()
            }
        }
    }

    fn millis_to_week_and_seconds(time: i64) -> BinnedTime {
        Self::millis_to_week_and_seconds_(Duration::milliseconds(time))
    }

    fn millis_to_week_and_seconds_(mut duration: Duration) -> BinnedTime {
        let weeks = duration.whole_weeks();
        let just_the_weeks = Duration::weeks(weeks);
        duration -= just_the_weeks;

        BinnedTime {
            bin: weeks,
            offset: TimeUnits::Seconds(duration.whole_seconds() as i128),
        }
    }

    fn millis_to_day_and_millis(time: i64) -> BinnedTime {
        Self::millis_to_day_and_millis_(Duration::milliseconds(time))
    }

    fn millis_to_day_and_millis_(mut duration: Duration) -> BinnedTime {
        let days = duration.whole_days();
        let just_the_days = Duration::days(days);
        duration -= just_the_days;

        BinnedTime {
            bin: days,
            offset: TimeUnits::Milliseconds(duration.whole_milliseconds()),
        }
    }

    fn millis_to_month_and_seconds(time: i64) -> BinnedTime {
        Self::millis_to_month_and_seconds_(Duration::milliseconds(time))
    }

    fn millis_to_month_and_seconds_(mut duration: Duration) -> BinnedTime {
        let months: i64 = duration.whole_days() / DAYS_IN_MONTH;
        let just_the_months = Duration::days(months * DAYS_IN_MONTH);
        duration -= just_the_months;
        BinnedTime {
            bin: months,
            offset: TimeUnits::Seconds(duration.whole_seconds() as i128),
        }
    }

    fn millis_to_year_and_minutes(time: i64) -> BinnedTime {
        Self::millis_to_year_and_minutes_(Duration::milliseconds(time))
    }

    fn millis_to_year_and_minutes_(mut duration: Duration) -> BinnedTime {
        let years = duration.whole_weeks() / WEEKS_IN_YEAR;

        let just_the_year = Duration::days(years * WEEKS_IN_YEAR);

        duration -= just_the_year;

        BinnedTime {
            bin: years,
            offset: TimeUnits::Minutes(duration.whole_minutes() as i128),
        }
    }
}

/// The period of time in a bin.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TimePeriod {
    /// A TimePeriod of One day increments.
    Day,
    /// A TimePeriod of One week increments.
    Week,
    /// A TimePeriod of One Month increments.
    Month,
    /// A Time Period of one Year increments.
    Year,
}

/// The units of the offset
#[derive(Debug, PartialEq)]
pub enum TimeUnits {
    /// The offset is in milliseconds.
    Milliseconds(i128),
    /// The offset is in seconds.
    Seconds(i128),
    /// The offset is in minutes.
    Minutes(i128),
}

impl TimeUnits {
    /// The number of milliseconds/seconds/minutes.
    #[must_use]
    pub fn num(self) -> i128 {
        match self {
            TimeUnits::Milliseconds(n) | TimeUnits::Seconds(n) | TimeUnits::Minutes(n) => n,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn binned_time_to_time(period: TimePeriod, binned_time: BinnedTime) -> i64 {
        let bin_dur = match period {
            TimePeriod::Day => Duration::days(binned_time.bin),
            TimePeriod::Week => Duration::weeks(binned_time.bin),
            TimePeriod::Month => Duration::days(binned_time.bin * DAYS_IN_MONTH),
            TimePeriod::Year => Duration::weeks(binned_time.bin * WEEKS_IN_YEAR),
        };

        let offset_dur = match period {
            TimePeriod::Day => Duration::milliseconds(binned_time.offset.num() as i64),
            TimePeriod::Week => Duration::seconds(binned_time.offset.num() as i64),
            TimePeriod::Month => Duration::seconds(binned_time.offset.num() as i64),
            TimePeriod::Year => Duration::minutes(binned_time.offset.num() as i64),
        };

        (bin_dur + offset_dur).whole_milliseconds() as i64
    }

    #[quickcheck]
    fn milliseconds_as_binned_day_is_millis(time: i64) -> bool {
        let binned = BinnedTime::from_millis(TimePeriod::Day, time);

        binned_time_to_time(TimePeriod::Day, binned) == time
    }

    #[quickcheck]
    fn milliseconds_as_binned_week_is_millis(time: i64) -> bool {
        let binned = BinnedTime::from_millis(TimePeriod::Week, time);

        binned_time_to_time(TimePeriod::Week, binned)
            == Duration::seconds(Duration::milliseconds(time).whole_seconds()).whole_milliseconds()
                as i64
    }

    #[quickcheck]
    fn milliseconds_as_binned_month_is_millis(time: i64) -> bool {
        let binned = BinnedTime::from_millis(TimePeriod::Month, time);

        binned_time_to_time(TimePeriod::Month, binned)
            == Duration::seconds(Duration::milliseconds(time).whole_seconds()).whole_milliseconds()
                as i64
    }

    #[quickcheck]
    fn milliseconds_as_binned_year_is_millis(time: i64) -> bool {
        let binned = BinnedTime::from_millis(TimePeriod::Year, time);

        binned_time_to_time(TimePeriod::Year, binned)
            == Duration::minutes(Duration::milliseconds(time).whole_minutes()).whole_milliseconds()
                as i64
    }
}
