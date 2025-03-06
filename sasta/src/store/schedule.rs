use std::str::FromStr;

use chrono::{DateTime, Local};
use cron::Schedule as CronSchedule;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone)]
enum ScheduledItem {
    Schedule {
        start: CronSchedule,
        end: CronSchedule,
        playlist: Uuid,
    },
    Fallback(Uuid),
}

#[derive(Clone)]
enum NextMoment {
    /// Next moment which changes active playlist
    Moment(Moment),
    /// Moment found, but does not change active playlist, a moment from tested scheduled time may exist
    Continue(DateTime<Local>),
    /// No future scheduled times exists, a moment will not be found
    Exhausted,
}

impl From<&NextMoment> for bool {
    fn from(value: &NextMoment) -> Self {
        match value {
            NextMoment::Exhausted => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub name: String,
    pub playlist: Uuid,
    schedules: Vec<ScheduledItem>,
}

impl Schedule {
    pub fn new(
        name: String,
        schedules_input: Vec<ScheduledPlaylistInput>,
        playlist: Uuid,
    ) -> Result<Self, String> {
        let mut schedules = Vec::with_capacity(schedules_input.len());

        for s in schedules_input {
            schedules.push(ScheduledItem::Schedule {
                start: Self::str_to_cron(&s.start)?,
                end: Self::str_to_cron(&s.end)?,
                playlist: s.playlist,
            })
        }
        schedules.push(ScheduledItem::Fallback(playlist));

        Ok(Schedule {
            name,
            playlist,
            schedules,
        })
    }

    fn str_to_cron(s: &str) -> Result<CronSchedule, String> {
        match CronSchedule::from_str(s) {
            Ok(s) => Ok(s),
            Err(_) => Err(format!("invalid cron expression '{}'", s)),
        }
    }

    /// Provides last scheduled time from given `DateTime` including the current time if applicable.
    fn previous_time(from: &DateTime<Local>, schedule: &CronSchedule) -> Option<DateTime<Local>> {
        match schedule.after(from).next_back() {
            // Check if current time is a scheduled moment
            _ if schedule.includes(from.clone()) => Some(from.clone()),
            // Check last schedules moment if any
            Some(back) if &back <= from => Some(back),
            // No scheduled moments have existed before
            _ => None,
        }
    }

    /// Get the current active playlist in schedule at the provided point in time
    pub fn current_playlist(&self, time: &DateTime<Local>) -> Uuid {
        self.schedules
            .iter()
            .find_map(|schedule| {
                // A branch returning a Some signals last scheduled moment was a start action, None is used otherwise.
                // find_map returns the value of the first closure to return a Some, and since all Vectors in Scheduled
                // include a fallback resulting in a guaranteed Some, unwrap may safely be used to avoid returning an Option
                match schedule {
                    ScheduledItem::Schedule {
                        start,
                        end,
                        playlist,
                    } => {
                        let (last_start, last_end) = (
                            Self::previous_time(time, start),
                            Self::previous_time(time, end),
                        );

                        if let (Some(last_start), Some(last_end)) = (last_start, last_end) {
                            // If both previous start and end moments were found, return Some if the most recent one was a start action
                            match last_start.cmp(&last_end) {
                                std::cmp::Ordering::Greater => Some(playlist.clone()),
                                std::cmp::Ordering::Less => None,
                                std::cmp::Ordering::Equal => {
                                    warn!(
                                        "Start and end action happening at the same moment in schedule {} playlist {playlist} ({})", 
                                        self.name,
                                        last_start.to_string()
                                    );
                                    None
                                }
                            }
                        } else {
                            // If at most one previous moment was found, return Some if it was a last_start
                            last_start.and_then(|_| Some(playlist.clone()))
                        }
                    }
                    ScheduledItem::Fallback(uuid) => Some(uuid.clone()),
                }
            })
            .unwrap()
    }

    // fn get_fallback(&self) -> Uuid {
    //     if let ScheduledItem::Fallback(uuid) = self.schedules.last().unwrap() {
    //         uuid.clone()
    //     } else {
    //         panic!("Last element should always be a fallback: {:#?}", self.schedules);
    //     }
    // }

    /// Returns next scheduled moment if any
    ///
    /// Does not return scheduled moments at the exact time passed as argument
    pub fn next_schedule(&self, from: &DateTime<Local>) -> Option<Moment> {
        // Vector with closures returning the next scheduled time
        let mut future_moments = self
            .schedules
            .iter()
            .filter_map(|schedule| {
                match schedule {
                    ScheduledItem::Schedule {
                        start,
                        end,
                        playlist,
                    } => {
                        let (mut next_start_iter, mut next_end_iter) =
                            (start.after(from).peekable(), end.after(from).peekable());

                        let current_playlist = self.current_playlist(&from);

                        Some(move || {
                            let time_opt = if &current_playlist == playlist {
                                // Only use the end scheduled times if current schedule's playlist is already active
                                next_end_iter.next()
                            } else {
                                // Consume item from the iterator which holds the closest point in time without modifying the other iterator
                                let (next_start, next_end) =
                                    (next_start_iter.peek(), next_end_iter.peek());
                                match (next_start, next_end) {
                                    (None, None) => None,
                                    (Some(_), None) => next_start_iter.next(),
                                    (None, Some(_)) => next_end_iter.next(),
                                    (Some(s), Some(e)) => match s.cmp(e) {
                                        std::cmp::Ordering::Less => next_start_iter.next(),
                                        std::cmp::Ordering::Greater => next_end_iter.next(),
                                        std::cmp::Ordering::Equal => {
                                            //TODO: Actual error handling here; extend enum with error type
                                            error!("Schedule {} has a start and end time at the same timestamp with the scheduled playlist {}. Marking playlist schedule as exhausted to avoid issues. TODO: Better error handling here", self.name, playlist);
                                            return NextMoment::Exhausted;
                                        }
                                    },
                                }
                            };

                            if let Some(time) = time_opt {
                                let playlist_at_moment = self.current_playlist(&time);
                                // Return a moment if schedule becomes active at time
                                if playlist_at_moment != current_playlist {
                                    return NextMoment::Moment(Moment {
                                        time,
                                        playlist: playlist_at_moment,
                                    });
                                }
                                // Return a continue if the Moment found was overshadowed by another Schedule
                                return NextMoment::Continue(time);
                            }
                            // Return if no future schedules times exists for schedule
                            NextMoment::Exhausted
                        })
                    }
                    // Fallback is not used in this function, ignore
                    ScheduledItem::Fallback(_) => None,
                }
            })
            // Create tuple structure where first value is the last result from the closure and the second is the closure itself
            // All start with with Continue result since a new result should be calculated in the loop
            .map(|f| (NextMoment::Continue(*from), f))
            .collect::<Vec<_>>();

        let max_date = DateTime::from(DateTime::<Local>::MAX_UTC);
        // Lowest timestamp for a moment found
        let mut closest_moment: DateTime<Local> = max_date;

        // Loop until the closest Moment to the from timestamp is found or until all schedules are exhausted
        loop {
            // Update the result of each item which held a previous Continue result
            for (res, f) in future_moments.iter_mut() {
                if let NextMoment::Continue(time) = res {
                    // Only update if the last result yielding a Continue was had a timestamp lower than the lowest found for a Moment yield
                    if time < &mut closest_moment {
                        // Deref and update res in tuple with new value
                        *res = f();
                        // If closure yielded a Moment, check if it is closer in time than the last found moment
                        if let NextMoment::Moment(m) = res {
                            // Should never be two equal Moments, so case does not need to be covered
                            closest_moment = closest_moment.min(m.time);
                        }
                    }
                }
            }

            // Check if all results are exhausted and exits if that is the case
            if let None = future_moments.iter().find(|(res, _)| res.into()) {
                return None;
            }

            // Checks the lowest timestamp of all results (trying to exclude Exhausted) and returns the result if
            // a res of type Moment was the closest in time. Since the vec `future_moments` is ordered by schedule
            // priority and `min_by_key` takes the first value if found equally minimum, the correct playlist is returned.
            // Should not even be an issue though since an overshadowed schedule (only was to get two equal Moments) would
            // result in an Continue result from closure since it is overshadowed with lower priority
            if let Some((NextMoment::Moment(m), _)) =
                future_moments.iter().min_by_key(|(res, _)| match res {
                    NextMoment::Moment(m) => m.time,
                    NextMoment::Continue(t) => *t,
                    NextMoment::Exhausted => max_date,
                })
            {
                return Some(m.to_owned());
            }
        }
    }

    /// True if the Schedule has any scheduled playlists
    pub fn has_scheduled_playlists(&self) -> bool {
        return !self.schedules.is_empty();
    }

    /// Returns Vec of the Uuids of all playlists (fallback + any scheduled) which the schedule contains
    pub fn all_playlists(&self) -> Vec<&Uuid> {
        self.schedules
            .iter()
            .map(|s| match s {
                ScheduledItem::Schedule { playlist, .. } | ScheduledItem::Fallback(playlist) => {
                    playlist
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Moment {
    /// Scheduled time of change
    pub time: DateTime<Local>,
    // /// Uuid of schedule to be updated
    // schedule: Uuid,
    /// Uuid of playlist which will become active at moment
    pub playlist: Uuid,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ScheduleInput {
    pub name: String,
    pub scheduled: Option<Vec<ScheduledPlaylistInput>>,
    pub playlist: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone, TS, ToSchema)]
#[ts(export, export_to = "api_bindings/update/")]
pub struct ScheduledPlaylistInput {
    #[ts(type = "string")]
    pub playlist: Uuid,
    pub start: String,
    pub end: String,
}

impl<'de> Deserialize<'de> for Schedule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input = ScheduleInput::deserialize(deserializer)?;
        match Schedule::new(
            input.name,
            input.scheduled.unwrap_or(vec![]),
            input.playlist,
        ) {
            Ok(s) => Ok(s),
            Err(s) => Err(serde::de::Error::custom(s)),
        }
    }
}

impl Serialize for Schedule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ScheduleInput::serialize(&self.to_owned().into(), serializer)
    }
}

impl From<Schedule> for ScheduleInput {
    fn from(value: Schedule) -> Self {
        let fallback = if let Some(ScheduledItem::Fallback(uuid)) = value.schedules.last() {
            uuid
        } else {
            panic!("No Fallback as last value");
            // return Err(serde::ser::Error::custom("No Fallback as last value"))
        };

        let scheduled = {
            let vec = value
                .schedules
                .iter()
                .filter_map(|s| match s {
                    ScheduledItem::Schedule {
                        start,
                        end,
                        playlist,
                    } => Some(ScheduledPlaylistInput {
                        playlist: *playlist,
                        start: start.to_string(),
                        end: end.to_string(),
                    }),
                    ScheduledItem::Fallback(_) => None,
                })
                .collect::<Vec<_>>();

            (!vec.is_empty()).then_some(vec)
        };

        ScheduleInput {
            name: value.name.clone(),
            scheduled,
            playlist: *fallback,
        }
    }
}

#[cfg(test)]
mod test {
    use chrono::{Local, TimeZone};
    use uuid::Uuid;

    use crate::store::schedule::Moment;

    use super::{Schedule, ScheduledPlaylistInput};

    #[test]
    fn test_next_schedule_many_schedules_with_wildcards() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let scheduled2_uuid = Uuid::parse_str("d125a360-4e41-45d5-b6c7-ea471c542510").unwrap();
        let scheduled3_uuid = Uuid::parse_str("05cb41ee-463d-41ca-870b-606a54f45d59").unwrap();
        let scheduled4_uuid = Uuid::parse_str("cc3c59da-5499-4b64-98c7-ca0501163479").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let schedules = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                // Since only one path is checked of start and end, with the lesser next scheduled event picked, test that the end
                // here still gets picked as first moment, since start schedule needs to be exhausted for end can start. This test makes sure
                // that start is chosen instead of a moment at a later time.
                start: "* * 10 * * * *".to_string(),
                end: "0 * 14 * * * *".to_string(),
            },
            ScheduledPlaylistInput {
                playlist: scheduled2_uuid,
                start: "* 0 11 * * * *".to_string(),
                end: "0 * 15 * * * *".to_string(),
            },
            ScheduledPlaylistInput {
                playlist: scheduled3_uuid,
                start: "* * 12 * * * *".to_string(),
                end: "* 0 16 * * * *".to_string(),
            },
            ScheduledPlaylistInput {
                playlist: scheduled4_uuid,
                start: "0 * 13 * * * *".to_string(),
                end: "* 0 17 * * * *".to_string(),
            },
        ];
        let schedule: Schedule =
            Schedule::new("test".to_string(), schedules, default_uuid).unwrap();

        let first_schedule_start = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap(),
            playlist: scheduled_uuid,
        };

        let first_schedule_end = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap(),
            playlist: scheduled2_uuid,
        };

        assert_eq!(
            first_schedule_start,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap())
                .unwrap()
        );

        let second_schedule_end = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 0).unwrap(),
            playlist: scheduled3_uuid,
        };

        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap())
                .unwrap()
        );
        assert_eq!(
            second_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap())
                .unwrap()
        );
        assert_eq!(
            second_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap())
                .unwrap()
        );

        let forth_schedule_end = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 17, 0, 0).unwrap(),
            playlist: default_uuid,
        };

        let first_schedule_start_next_day = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 19, 10, 0, 0).unwrap(),
            playlist: scheduled_uuid,
        };

        assert_eq!(
            forth_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 16, 59, 59).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_start_next_day,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 17, 0, 0).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_start_next_day,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 17, 0, 1).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn test_next_schedule_priority() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let scheduled2_uuid = Uuid::parse_str("d125a360-4e41-45d5-b6c7-ea471c542510").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let schedules = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                start: "0 0 10 * * * *".to_string(),
                end: "0 0 14 * * * *".to_string(),
            },
            ScheduledPlaylistInput {
                playlist: scheduled2_uuid,
                start: "0 0 10 * * * *".to_string(),
                end: "0 0 14 * * * *".to_string(),
            },
        ];
        let schedule: Schedule =
            Schedule::new("test".to_string(), schedules, default_uuid).unwrap();

        assert_eq!(
            Moment {
                time: Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap(),
                playlist: scheduled_uuid
            },
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap())
                .unwrap()
        );

        let first_schedule_end = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap(),
            playlist: default_uuid,
        };

        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn test_next_schedule() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let scheduled2_uuid = Uuid::parse_str("d125a360-4e41-45d5-b6c7-ea471c542510").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let schedules = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                start: "0 0 10 * * * *".to_string(),
                end: "0 0 14 * * * *".to_string(),
            },
            ScheduledPlaylistInput {
                playlist: scheduled2_uuid,
                start: "0 0 11 * * * *".to_string(),
                end: "0 0 15 * * * *".to_string(),
            },
        ];
        let schedule: Schedule =
            Schedule::new("test".to_string(), schedules, default_uuid).unwrap();

        assert_eq!(
            Moment {
                time: Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap(),
                playlist: scheduled_uuid
            },
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap())
                .unwrap()
        );

        let first_schedule_end = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap(),
            playlist: scheduled2_uuid,
        };

        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap())
                .unwrap()
        );

        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 59, 59).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 11, 0, 0).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 11, 0, 1).unwrap())
                .unwrap()
        );

        let second_schedule_end = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 0).unwrap(),
            playlist: default_uuid,
        };

        assert_eq!(
            first_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap())
                .unwrap()
        );
        assert_eq!(
            second_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap())
                .unwrap()
        );
        assert_eq!(
            second_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap())
                .unwrap()
        );

        let first_schedule_start = Moment {
            time: Local.with_ymd_and_hms(2023, 4, 19, 10, 0, 0).unwrap(),
            playlist: scheduled_uuid,
        };

        assert_eq!(
            second_schedule_end,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 59, 59).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_start,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 0).unwrap())
                .unwrap()
        );
        assert_eq!(
            first_schedule_start,
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 1).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn test_next_schedule_specific_date() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let scheduled2_uuid = Uuid::parse_str("d125a360-4e41-45d5-b6c7-ea471c542510").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let schedules = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                start: "0 0 10 18 4 * 2023".to_string(),
                end: "0 0 14 18 4 * 2023".to_string(),
            },
            ScheduledPlaylistInput {
                playlist: scheduled2_uuid,
                start: "0 0 11 18 4 * 2023".to_string(),
                end: "0 0 15 18 4 * 2023".to_string(),
            },
        ];
        let schedule: Schedule =
            Schedule::new("test".to_string(), schedules, default_uuid).unwrap();

        assert_eq!(
            Moment {
                time: Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap(),
                playlist: scheduled_uuid
            },
            schedule
                .next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap())
                .unwrap()
        );

        let first_schedule_end = Some(Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap(),
            playlist: scheduled2_uuid,
        });

        assert_eq!(
            first_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap())
        );
        assert_eq!(
            first_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap())
        );

        assert_eq!(
            first_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 59, 59).unwrap())
        );
        assert_eq!(
            first_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 11, 0, 0).unwrap())
        );
        assert_eq!(
            first_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 11, 0, 1).unwrap())
        );

        let second_schedule_end = Some(Moment {
            time: Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 0).unwrap(),
            playlist: default_uuid,
        });

        assert_eq!(
            first_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap())
        );
        assert_eq!(
            second_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap())
        );
        assert_eq!(
            second_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap())
        );

        assert_eq!(
            second_schedule_end,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 59, 59).unwrap())
        );
        assert_eq!(
            None,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 0).unwrap())
        );
        assert_eq!(
            None,
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 1).unwrap())
        );
    }

    #[test]
    fn test_current_playlist_specific_date() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let schedules = vec![ScheduledPlaylistInput {
            playlist: scheduled_uuid,
            start: "0 0 10 18 4 * 2023".to_string(),
            end: "0 0 14 18 4 * 2023".to_string(),
        }];
        let schedule: Schedule =
            Schedule::new("test".to_string(), schedules, default_uuid).unwrap();

        assert_eq!(
            default_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap())
        );
        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap())
        );
        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap())
        );

        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap())
        );
        assert_eq!(
            default_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap())
        );
        assert_eq!(
            default_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap())
        );
    }

    // Only works with https://github.com/zslayton/cron/pull/116 at this time. Hopefully it gets merged
    #[test]
    fn test_next_after_past_date_next_year() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let schedules = vec![ScheduledPlaylistInput {
            playlist: scheduled_uuid,
            start: "0 0 10 * * * 2025/1".to_string(),
            end: "0 0 11 * * * 2025/1".to_string(),
        }];
        let schedule: Schedule =
            Schedule::new("test".to_string(), schedules, default_uuid).unwrap();

        assert_eq!(
            default_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2024, 8, 8, 13, 42, 00).unwrap())
        );
        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2025, 1, 1, 10, 42, 00).unwrap())
        );

        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap())
        );
        assert_eq!(
            Some(Moment {
                time: Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap(),
                playlist: scheduled_uuid
            }),
            schedule.next_schedule(&Local.with_ymd_and_hms(2024, 8, 8, 13, 42, 00).unwrap())
        );
    }

    #[test]
    fn test_current_playlist() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let playlist = vec![ScheduledPlaylistInput {
            playlist: scheduled_uuid,
            start: "0 * 10 * * * *".to_string(),
            end: "0 0 14 * * * *".to_string(),
        }];
        let schedule: Schedule = Schedule::new("test".to_string(), playlist, default_uuid).unwrap();

        assert_eq!(
            default_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap())
        );
        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap())
        );
        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap())
        );

        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap())
        );
        assert_eq!(
            default_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap())
        );
        assert_eq!(
            default_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap())
        );
    }

    #[test]
    fn test_invalid_date() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let playlist = vec![ScheduledPlaylistInput {
            playlist: scheduled_uuid,
            start: "0 * 10 32 10 * *".to_string(),
            end: "0 0 14 32 10 * *".to_string(),
        }];
        assert!(Schedule::new("test".into(), playlist, default_uuid).is_err());
    }

    #[test]
    fn test_current_playlist_smooth_transition() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let scheduled2_uuid = Uuid::parse_str("d125a360-4e41-45d5-b6c7-ea471c542510").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let mut playlist = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                start: "0 0 10 * * * *".to_string(),
                end: "0 0 14 * * * *".to_string(),
            },
            ScheduledPlaylistInput {
                playlist: scheduled2_uuid,
                start: "0 0 14 * * * *".to_string(),
                end: "0 0 18 * * * *".to_string(),
            },
        ];
        let schedule: Schedule =
            Schedule::new("test".to_string(), playlist.clone(), default_uuid).unwrap();

        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap())
        );
        assert_eq!(
            scheduled2_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap())
        );
        assert_eq!(
            scheduled2_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap())
        );

        playlist.reverse();
        let schedule: Schedule = Schedule::new("test".to_string(), playlist, default_uuid).unwrap();

        assert_eq!(
            scheduled_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap())
        );
        assert_eq!(
            scheduled2_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap())
        );
        assert_eq!(
            scheduled2_uuid,
            schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap())
        );
    }
}
