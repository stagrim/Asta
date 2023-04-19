use std::str::FromStr;

use chrono::{DateTime, Local};
use serde::Deserialize;
use uuid::Uuid;
use cron::Schedule as CronSchedule;

#[derive(Debug)]
enum ScheduledItem {
    Schedule {
        start: CronSchedule,
        end: CronSchedule,
        playlist: Uuid
    },
    Fallback (Uuid)
}

#[derive(Debug)]
pub struct Schedule {
    pub name: String,
    pub playlist: Uuid,
    schedules: Vec<ScheduledItem>
}

impl Schedule {
    fn new(name: String, schedules: Vec<ScheduledPlaylistInput>, fallback: Uuid) -> Self {
        Schedule {
            name,
            playlist: fallback,
            schedules:
                schedules.iter().map(|s| ScheduledItem::Schedule { 
                    start: CronSchedule::from_str(&s.start).unwrap(),
                    end: CronSchedule::from_str(&s.end).unwrap(),
                    playlist: s.playlist
                })
                .chain(vec![ScheduledItem::Fallback(fallback)].into_iter())
                .collect()
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
            _ => None
        }
    }

    /// Get the current active playlist in schedule at the provided point in time
    pub fn current_playlist(&self, time: &DateTime<Local>) -> Uuid {
        self.schedules.iter().find_map(|schedule| {
            // A branch returning a Some signals last scheduled moment was a start action, None is used otherwise.
            // find_map returns the value of the first closure to return a Some, and since all Vectors in Scheduled
            // include a fallback resulting in a guaranteed Some, unwrap may safely be used to avoid returning an Option
            match schedule {
                ScheduledItem::Schedule { start, end, playlist } => {
                    let (last_start, last_end) =
                        (Self::previous_time(time, start), Self::previous_time(time, end));
                    
                    if let (Some(last_start), Some(last_end)) = (last_start, last_end) {
                        // If both previous start and end moments were found, return Some if the most recent one was a start action
                        match last_start.cmp(&last_end) {
                            std::cmp::Ordering::Greater => Some(playlist.clone()),
                            std::cmp::Ordering::Less => None,
                            std::cmp::Ordering::Equal => panic!("Start and end action happening at the same moment ({})", last_start.to_string()),
                        }
                    } else {
                        // If at most one previous moment was found, return Some if it was a last_start
                        last_start.and_then(|_| Some(playlist.clone()))
                    }
                },
                ScheduledItem::Fallback(uuid) => Some(uuid.clone()),
            }
        }).unwrap()
    }

    fn get_fallback(&self) -> Uuid {
        if let ScheduledItem::Fallback(uuid) = self.schedules.last().unwrap() {
            uuid.clone()
        } else {
            panic!("Last element should always be a fallback: {:#?}", self.schedules);
        }
    }

    /// Returns next scheduled moment if any
    /// 
    /// Does not return scheduled moments at the exact time passed as argument
    pub fn next_schedule(&self, from: &DateTime<Local>) -> Option<Moment> {
        let mut moments = self.schedules.iter()
            .filter_map(|schedule| {
                match schedule {
                    ScheduledItem::Schedule { start, end, playlist } => {
                        let (mut next_start_iter, mut next_end_iter) =
                            (start.after(from), end.after(from));
                        
                        let current_playlist = self.current_playlist(&from);
                        Some(move || {
                            let (next_start, next_end) = (next_start_iter.next(), next_end_iter.next());
                            let a = if let (Some(next_start), Some(next_end)) = (next_start, next_end) {
                                // If both previous start and end moments were found, return Some if the most recent one was a start action
                                if next_start == next_end {
                                    panic!("Start and end action happening at the same moment ({})", next_start.to_string());
                                }
                                Some(next_start.min(next_end))
                            } else {
                                next_start.or(next_end)
                            };
                            if let Some(time) = a {
                                let playlist_at_moment = self.current_playlist(&time);
                                if playlist_at_moment != current_playlist {
                                    return Ok(Moment { time, playlist: playlist_at_moment })
                                }
                                return Err(Some(time))
                            }
                            Err(None)
                        })
                    },
                    ScheduledItem::Fallback(_) => None,
                }
            });
        
        loop {
            let mut closest_time =
                (&mut moments).filter_map(|mut f| match f() {
                    Ok(m) => Some(Ok(m)),
                    Err(Some(t)) => Some(Err((f, t))),
                    Err(None) => None,
                });
            
            if let Some(Ok(m)) = closest_time.find(|r| r.is_ok()) {
                return Some(m)
            }

            if closest_time.count() == 0 {
                return None
            }
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Moment {
    /// Scheduled time of change
    pub time: DateTime<Local>,
    // /// Uuid of schedule to be updated 
    // schedule: Uuid,
    /// Uuid of playlist which will become active at moment
    pub playlist: Uuid,
}

#[derive(Deserialize, Debug)]
struct ScheduleInput {
    name: String,
    scheduled: Option<Vec<ScheduledPlaylistInput>>,
    playlist: Uuid
}

#[derive(Deserialize, Debug, Clone)]
struct ScheduledPlaylistInput {
    playlist: Uuid,
    start: String,
    end: String
}

impl<'de> Deserialize<'de> for Schedule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let input = ScheduleInput::deserialize(deserializer).unwrap();
        Ok(Schedule::new(input.name, input.scheduled.unwrap_or(vec![]), input.playlist))
    }
}

#[cfg(test)]
mod test {
    use chrono::{Local, TimeZone};
    use uuid::Uuid;

    use crate::store::schedule::Moment;

    use super::{Schedule, ScheduledPlaylistInput};

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
            }
        ];
        let schedule: Schedule = Schedule::new("test".to_string(), schedules, default_uuid);
        
        assert_eq!(
            Moment { time: Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap(), playlist: scheduled_uuid },
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap()).unwrap()
        );

        let first_schedule_end =
            Moment { time: Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap(), playlist: scheduled2_uuid };

        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap()).unwrap());
        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap()).unwrap());

        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 59, 59).unwrap()).unwrap());
        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 11, 0, 0).unwrap()).unwrap());
        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 11, 0, 1).unwrap()).unwrap());
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
            }
        ];
        let schedule: Schedule = Schedule::new("test".to_string(), schedules, default_uuid);
        
        assert_eq!(
            Moment { time: Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap(), playlist: scheduled_uuid },
            schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap()).unwrap()
        );

        let first_schedule_end =
            Some(Moment { time: Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap(), playlist: scheduled2_uuid });

        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap()));
        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap()));

        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 10, 59, 59).unwrap()));
        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 11, 0, 0).unwrap()));
        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 11, 0, 1).unwrap()));

        let second_schedule_end =
            Some(Moment { time: Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 0).unwrap(), playlist: default_uuid });

        assert_eq!(first_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap()));
        assert_eq!(second_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap()));
        assert_eq!(second_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap()));

        assert_eq!(second_schedule_end, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 14, 59, 59).unwrap()));
        assert_eq!(None, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 0).unwrap()));
        assert_eq!(None, schedule.next_schedule(&Local.with_ymd_and_hms(2023, 4, 18, 15, 0, 1).unwrap()));
    }

    #[test]
    fn test_current_playlist_specific_date() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let schedules = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                start: "0 0 10 18 4 * 2023".to_string(),
                end: "0 0 14 18 4 * 2023".to_string(),
            }
        ];
        let schedule: Schedule = Schedule::new("test".to_string(), schedules, default_uuid);

        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap()));
        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap()));
        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap()));

        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap()));
        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap()));
        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap()));
    }

    #[test]
    fn test_current_playlist() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let playlist = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                start: "0 0 10 * * * *".to_string(),
                end: "0 0 14 * * * *".to_string(),
            }
        ];
        let schedule: Schedule = Schedule::new("test".to_string(), playlist, default_uuid);

        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap()));
        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap()));
        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap()));

        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap()));
        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap()));
        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap()));
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
            }
        ];
        let schedule: Schedule = Schedule::new("test".to_string(), playlist.clone(), default_uuid);

        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap()));
        assert_eq!(scheduled2_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap()));
        assert_eq!(scheduled2_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap()));

        playlist.reverse();
        let schedule: Schedule = Schedule::new("test".to_string(), playlist, default_uuid);

        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap()));
        assert_eq!(scheduled2_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap()));
        assert_eq!(scheduled2_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap()));
    }
}
