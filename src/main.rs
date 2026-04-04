mod caldav;
mod carddav;
mod commands;
mod config;
mod error;
mod jmap;
mod mcp;
mod models;
pub mod util;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use models::Output;
use std::io;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "fastmail-cli")]
#[command(version, about = "CLI for Fastmail's JMAP API", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Fastmail API token
    Auth {
        /// API token from Fastmail settings
        token: String,
    },

    /// List resources
    #[command(subcommand)]
    List(ListCommands),

    /// Get a specific email by ID
    Get {
        /// Email ID
        email_id: String,
    },

    /// Get all emails in a thread/conversation
    Thread {
        /// Email ID (will fetch entire thread this email belongs to)
        email_id: String,
    },

    /// Search emails with JMAP filters
    Search {
        /// Full-text search (from, to, cc, bcc, subject, body)
        #[arg(short, long)]
        text: Option<String>,

        /// Filter by From header
        #[arg(long)]
        from: Option<String>,

        /// Filter by To header
        #[arg(long)]
        to: Option<String>,

        /// Filter by Cc header
        #[arg(long)]
        cc: Option<String>,

        /// Filter by Bcc header
        #[arg(long)]
        bcc: Option<String>,

        /// Filter by Subject
        #[arg(long)]
        subject: Option<String>,

        /// Filter by body content
        #[arg(long)]
        body: Option<String>,

        /// Filter by mailbox name
        #[arg(short, long)]
        mailbox: Option<String>,

        /// Only emails with attachments
        #[arg(long)]
        has_attachment: bool,

        /// Minimum email size in bytes
        #[arg(long)]
        min_size: Option<u32>,

        /// Maximum email size in bytes
        #[arg(long)]
        max_size: Option<u32>,

        /// Emails received before date (ISO 8601, e.g., 2024-01-01)
        #[arg(long)]
        before: Option<String>,

        /// Emails received on or after date (ISO 8601, e.g., 2024-01-01)
        #[arg(long)]
        after: Option<String>,

        /// Only unread emails
        #[arg(long)]
        unread: bool,

        /// Only flagged/starred emails
        #[arg(long)]
        flagged: bool,

        /// Maximum results
        #[arg(short, long, default_value = "50")]
        limit: u32,
    },

    /// Send an email
    Send {
        /// Recipient(s), comma-separated
        #[arg(long)]
        to: String,

        /// Subject line
        #[arg(long)]
        subject: String,

        /// Email body (plain text)
        #[arg(long)]
        body: String,

        /// CC recipient(s), comma-separated
        #[arg(long)]
        cc: Option<String>,

        /// BCC recipient(s), comma-separated
        #[arg(long)]
        bcc: Option<String>,

        /// In-Reply-To message ID (for threading)
        #[arg(long)]
        reply_to: Option<String>,

        /// Send from a specific identity (email address). Use `list identities` to see available.
        #[arg(long)]
        from: Option<String>,

        /// Save as draft instead of sending
        #[arg(long)]
        draft: bool,
    },

    /// Move email to a mailbox
    Move {
        /// Email ID
        email_id: String,

        /// Destination mailbox name
        #[arg(long)]
        to: String,
    },

    /// Mark email as spam
    Spam {
        /// Email ID
        email_id: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Mark email as read or unread
    MarkRead {
        /// Email ID
        email_id: String,

        /// Mark as unread instead of read
        #[arg(long)]
        unread: bool,
    },

    /// Download attachments from an email
    Download {
        /// Email ID
        email_id: String,

        /// Output directory (default: current directory)
        #[arg(short, long)]
        output: Option<String>,

        /// Output format: raw (save files) or json (extract text)
        #[arg(short, long)]
        format: Option<String>,

        /// Max size for images (e.g., 500K, 1M). Images larger than this are resized.
        #[arg(long)]
        max_size: Option<String>,
    },

    /// Reply to an email
    Reply {
        /// Email ID to reply to
        email_id: String,

        /// Reply body (plain text)
        #[arg(long)]
        body: String,

        /// Reply to all recipients
        #[arg(long)]
        all: bool,

        /// Additional CC recipient(s), comma-separated
        #[arg(long)]
        cc: Option<String>,

        /// BCC recipient(s), comma-separated
        #[arg(long)]
        bcc: Option<String>,

        /// Send from a specific identity (email address). Use `list identities` to see available.
        #[arg(long)]
        from: Option<String>,

        /// Save as draft instead of sending
        #[arg(long)]
        draft: bool,
    },

    /// Forward an email
    Forward {
        /// Email ID to forward
        email_id: String,

        /// Recipient(s), comma-separated
        #[arg(long)]
        to: String,

        /// Message to include before forwarded content
        #[arg(long, default_value = "")]
        body: String,

        /// CC recipient(s), comma-separated
        #[arg(long)]
        cc: Option<String>,

        /// BCC recipient(s), comma-separated
        #[arg(long)]
        bcc: Option<String>,

        /// Send from a specific identity (email address). Use `list identities` to see available.
        #[arg(long)]
        from: Option<String>,

        /// Save as draft instead of sending
        #[arg(long)]
        draft: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Manage masked email addresses
    #[command(subcommand)]
    Masked(MaskedCommands),

    /// Manage contacts via CardDAV
    #[command(subcommand)]
    Contacts(ContactsCommands),

    /// Manage calendars via CalDAV
    #[command(subcommand)]
    Calendars(CalendarsCommands),

    /// Manage calendar events via CalDAV
    #[command(subcommand)]
    Events(EventsCommands),

    /// Run as MCP (Model Context Protocol) server for Claude integration
    Mcp,
}

#[derive(Subcommand)]
enum MaskedCommands {
    /// List all masked email addresses
    List,

    /// Create a new masked email address
    Create {
        /// Domain this masked email is for (e.g., https://example.com)
        #[arg(long)]
        domain: Option<String>,

        /// Description for the masked email
        #[arg(long)]
        description: Option<String>,

        /// Custom prefix for the email address (max 64 chars, a-z/0-9/underscore)
        #[arg(long)]
        prefix: Option<String>,
    },

    /// Enable a masked email address
    Enable {
        /// Masked email ID
        id: String,
    },

    /// Disable a masked email address
    Disable {
        /// Masked email ID
        id: String,
    },

    /// Delete a masked email address
    Delete {
        /// Masked email ID
        id: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum ListCommands {
    /// List mailboxes (folders)
    Mailboxes,

    /// List emails in a mailbox
    Emails {
        /// Mailbox name (default: INBOX)
        #[arg(short, long, default_value = "INBOX")]
        mailbox: String,

        /// Maximum results
        #[arg(short, long, default_value = "50")]
        limit: u32,
    },

    /// List sender identities (for use with --from)
    Identities,
}

#[derive(Subcommand)]
enum ContactsCommands {
    /// List all contacts
    List,

    /// Search contacts by name or email
    Search {
        /// Search query
        query: String,
    },

    /// Create a new contact
    Create {
        /// Full name
        #[arg(long)]
        name: String,

        /// Email address
        #[arg(long)]
        email: Option<String>,

        /// Phone number
        #[arg(long)]
        phone: Option<String>,

        /// Organization / company
        #[arg(long)]
        organization: Option<String>,

        /// Job title
        #[arg(long)]
        title: Option<String>,

        /// Street address
        #[arg(long)]
        address: Option<String>,

        /// Notes
        #[arg(long)]
        notes: Option<String>,
    },

    /// Update an existing contact
    Update {
        /// Contact ID
        contact_id: String,

        /// Full name
        #[arg(long)]
        name: Option<String>,

        /// Email address
        #[arg(long)]
        email: Option<String>,

        /// Phone number
        #[arg(long)]
        phone: Option<String>,

        /// Organization / company
        #[arg(long)]
        organization: Option<String>,

        /// Job title
        #[arg(long)]
        title: Option<String>,

        /// Street address
        #[arg(long)]
        address: Option<String>,

        /// Notes
        #[arg(long)]
        notes: Option<String>,
    },

    /// Delete an existing contact
    Delete {
        /// Contact ID
        contact_id: String,

        /// Confirm deletion
        #[arg(long)]
        confirm: bool,

        /// Alias for --confirm
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum CalendarsCommands {
    /// List all calendars
    List,

    /// Create a new calendar
    Create {
        /// Calendar display name
        #[arg(long)]
        name: String,

        /// Optional calendar color (e.g. #3a87ad)
        #[arg(long)]
        color: Option<String>,
    },

    /// Update an existing calendar
    Update {
        /// Calendar ID
        calendar_id: String,

        /// Updated calendar name
        #[arg(long)]
        name: Option<String>,

        /// Updated calendar color
        #[arg(long)]
        color: Option<String>,
    },

    /// Delete an existing calendar
    Delete {
        /// Calendar ID
        calendar_id: String,

        /// Confirm deletion
        #[arg(long)]
        confirm: bool,

        /// Alias for --confirm
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum EventsCommands {
    /// List events. Defaults to future events for the rest of today; explicit range mode requires both --start and --end.
    List {
        /// Limit results to one calendar ID
        #[arg(long)]
        calendar: Option<String>,

        /// List the current week instead of the default today range
        #[arg(long)]
        week: bool,

        /// Explicit range start (YYYY-MM-DD or RFC3339). Requires --end.
        #[arg(long)]
        start: Option<String>,

        /// Explicit range end (YYYY-MM-DD or RFC3339). Requires --start.
        #[arg(long)]
        end: Option<String>,
    },

    /// Fetch one event by ID
    Get {
        /// Event ID (UID)
        event_id: String,

        /// Optional calendar ID hint
        #[arg(long)]
        calendar: Option<String>,
    },

    /// Create a new event
    Create {
        /// Target calendar ID. Defaults to the discovered primary/default calendar.
        #[arg(long)]
        calendar: Option<String>,

        /// Event title
        #[arg(long)]
        title: String,

        /// Start date/time: YYYY-MM-DD, YYYY-MM-DDTHH:MM[:SS], or RFC3339
        #[arg(long)]
        start: String,

        /// End date/time: YYYY-MM-DD, YYYY-MM-DDTHH:MM[:SS], or RFC3339
        #[arg(long)]
        end: String,

        /// Timezone for naive local datetimes
        #[arg(long)]
        timezone: Option<String>,

        /// Event location
        #[arg(long)]
        location: Option<String>,

        /// Event description
        #[arg(long)]
        description: Option<String>,

        /// Repeatable attendee email address
        #[arg(long)]
        attendee: Vec<String>,

        /// RRULE frequency, e.g. DAILY, WEEKLY, MONTHLY
        #[arg(long)]
        recurrence_freq: Option<String>,

        /// RRULE interval
        #[arg(long)]
        recurrence_interval: Option<u32>,

        /// RRULE count
        #[arg(long)]
        recurrence_count: Option<u32>,

        /// RRULE until (basic iCalendar value, e.g. 20260430T120000Z)
        #[arg(long)]
        recurrence_until: Option<String>,

        /// RRULE BYDAY values, repeatable (e.g. --recurrence-by-day MO)
        #[arg(long = "recurrence-by-day")]
        recurrence_by_day: Vec<String>,

        /// Reminder minutes before the event, repeatable
        #[arg(long = "reminder-minutes")]
        reminder_minutes: Vec<i32>,
    },

    /// Update an existing event
    Update {
        /// Event ID (UID)
        event_id: String,

        /// Optional calendar ID hint
        #[arg(long)]
        calendar: Option<String>,

        #[arg(long)]
        title: Option<String>,

        #[arg(long)]
        start: Option<String>,

        #[arg(long)]
        end: Option<String>,

        #[arg(long)]
        timezone: Option<String>,

        #[arg(long)]
        location: Option<String>,

        #[arg(long)]
        description: Option<String>,

        /// Replace attendees with this set. Repeatable.
        #[arg(long)]
        attendee: Vec<String>,

        /// Clear all attendees from the event. Conflicts with --attendee.
        #[arg(long, conflicts_with = "attendee")]
        clear_attendees: bool,

        #[arg(long)]
        recurrence_freq: Option<String>,

        #[arg(long)]
        recurrence_interval: Option<u32>,

        #[arg(long)]
        recurrence_count: Option<u32>,

        #[arg(long)]
        recurrence_until: Option<String>,

        #[arg(long = "recurrence-by-day")]
        recurrence_by_day: Vec<String>,

        /// Remove recurrence entirely
        #[arg(long)]
        clear_recurrence: bool,

        /// Replace reminders with this set. Repeatable.
        #[arg(long = "reminder-minutes")]
        reminder_minutes: Vec<i32>,

        /// Remove reminders entirely
        #[arg(long)]
        clear_reminders: bool,
    },

    /// Delete an existing event
    Delete {
        /// Event ID (UID)
        event_id: String,

        /// Optional calendar ID hint
        #[arg(long)]
        calendar: Option<String>,

        /// Confirm deletion
        #[arg(long)]
        confirm: bool,

        /// Alias for --confirm
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Auth { token } => commands::auth(&token).await,

        Commands::List(cmd) => match cmd {
            ListCommands::Mailboxes => commands::list_mailboxes().await,
            ListCommands::Emails { mailbox, limit } => commands::list_emails(&mailbox, limit).await,
            ListCommands::Identities => commands::list_identities().await,
        },

        Commands::Get { email_id } => commands::get_email(&email_id).await,

        Commands::Thread { email_id } => commands::get_thread(&email_id).await,

        Commands::Search {
            text,
            from,
            to,
            cc,
            bcc,
            subject,
            body,
            mailbox,
            has_attachment,
            min_size,
            max_size,
            before,
            after,
            unread,
            flagged,
            limit,
        } => {
            commands::search(
                commands::SearchFilter {
                    text,
                    from,
                    to,
                    cc,
                    bcc,
                    subject,
                    body,
                    mailbox,
                    has_attachment,
                    min_size,
                    max_size,
                    before,
                    after,
                    unread,
                    flagged,
                },
                limit,
            )
            .await
        }

        Commands::Send {
            to,
            subject,
            body,
            cc,
            bcc,
            reply_to,
            from,
            draft,
        } => {
            commands::send(
                &to,
                &subject,
                &body,
                reply_to.as_deref(),
                jmap::ComposeParams {
                    cc: cc.as_deref().map(util::parse_addresses).unwrap_or_default(),
                    bcc: bcc
                        .as_deref()
                        .map(util::parse_addresses)
                        .unwrap_or_default(),
                    from: from.as_deref(),
                    draft,
                },
            )
            .await
        }

        Commands::Move { email_id, to } => commands::move_email(&email_id, &to).await,

        Commands::Spam { email_id, yes } => {
            if !yes {
                Output::<()>::error("Confirmation required: pass -y to mark email as spam").print();
                anyhow::bail!("confirmation required");
            }
            commands::mark_spam(&email_id).await
        }

        Commands::MarkRead { email_id, unread } => commands::mark_read(&email_id, !unread).await,

        Commands::Download {
            email_id,
            output,
            format,
            max_size,
        } => {
            commands::download_attachment(
                &email_id,
                output.as_deref(),
                format.as_deref(),
                max_size.as_deref(),
            )
            .await
        }

        Commands::Reply {
            email_id,
            body,
            all,
            cc,
            bcc,
            from,
            draft,
        } => {
            commands::reply(
                &email_id,
                &body,
                all,
                jmap::ComposeParams {
                    cc: cc.as_deref().map(util::parse_addresses).unwrap_or_default(),
                    bcc: bcc
                        .as_deref()
                        .map(util::parse_addresses)
                        .unwrap_or_default(),
                    from: from.as_deref(),
                    draft,
                },
            )
            .await
        }

        Commands::Forward {
            email_id,
            to,
            body,
            cc,
            bcc,
            from,
            draft,
        } => {
            commands::forward(
                &email_id,
                &to,
                &body,
                jmap::ComposeParams {
                    cc: cc.as_deref().map(util::parse_addresses).unwrap_or_default(),
                    bcc: bcc
                        .as_deref()
                        .map(util::parse_addresses)
                        .unwrap_or_default(),
                    from: from.as_deref(),
                    draft,
                },
            )
            .await
        }

        Commands::Completions { shell } => {
            generate(
                shell,
                &mut Cli::command(),
                "fastmail-cli",
                &mut io::stdout(),
            );
            Ok(())
        }

        Commands::Masked(cmd) => match cmd {
            MaskedCommands::List => commands::list_masked_emails().await,
            MaskedCommands::Create {
                domain,
                description,
                prefix,
            } => {
                commands::create_masked_email(
                    domain.as_deref(),
                    description.as_deref(),
                    prefix.as_deref(),
                )
                .await
            }
            MaskedCommands::Enable { id } => commands::enable_masked_email(&id).await,
            MaskedCommands::Disable { id } => commands::disable_masked_email(&id).await,
            MaskedCommands::Delete { id, yes } => {
                if !yes {
                    Output::<()>::error("Confirmation required: pass -y to delete masked email").print();
                    anyhow::bail!("confirmation required");
                }
                commands::delete_masked_email(&id).await
            }
        },

        Commands::Contacts(cmd) => match cmd {
            ContactsCommands::List => commands::list_contacts().await,
            ContactsCommands::Search { query } => commands::search_contacts(&query).await,
            ContactsCommands::Create {
                name,
                email,
                phone,
                organization,
                title,
                address,
                notes,
            } => {
                commands::create_contact(commands::ContactInput {
                    name,
                    email,
                    phone,
                    organization,
                    title,
                    address,
                    notes,
                })
                .await
            }
            ContactsCommands::Update {
                contact_id,
                name,
                email,
                phone,
                organization,
                title,
                address,
                notes,
            } => {
                commands::update_contact(
                    &contact_id,
                    commands::ContactPatch {
                        name,
                        email,
                        phone,
                        organization,
                        title,
                        address,
                        notes,
                    },
                )
                .await
            }
            ContactsCommands::Delete {
                contact_id,
                confirm,
                yes,
            } => {
                if !(confirm || yes) {
                    Output::<()>::error("Confirmation required: pass --confirm to delete contact").print();
                    anyhow::bail!("confirmation required");
                }
                commands::delete_contact(&contact_id).await
            }
        },

        Commands::Calendars(cmd) => match cmd {
            CalendarsCommands::List => commands::list_calendars().await,
            CalendarsCommands::Create { name, color } => {
                commands::create_calendar(&name, color.as_deref()).await
            }
            CalendarsCommands::Update {
                calendar_id,
                name,
                color,
            } => commands::update_calendar(&calendar_id, name.as_deref(), color.as_deref()).await,
            CalendarsCommands::Delete {
                calendar_id,
                confirm,
                yes,
            } => {
                if !(confirm || yes) {
                    Output::<()>::error("Confirmation required: pass --confirm to delete calendar").print();
                    anyhow::bail!("confirmation required");
                }
                commands::delete_calendar(&calendar_id).await
            }
        },

        Commands::Events(cmd) => match cmd {
            EventsCommands::List {
                calendar,
                week,
                start,
                end,
            } => {
                commands::list_events(calendar.as_deref(), start.as_deref(), end.as_deref(), week)
                    .await
            }
            EventsCommands::Get { event_id, calendar } => {
                commands::get_event(&event_id, calendar.as_deref()).await
            }
            EventsCommands::Create {
                calendar,
                title,
                start,
                end,
                timezone,
                location,
                description,
                attendee,
                recurrence_freq,
                recurrence_interval,
                recurrence_count,
                recurrence_until,
                recurrence_by_day,
                reminder_minutes,
            } => {
                let recurrence = recurrence_freq.map(|frequency| crate::caldav::EventRecurrence {
                    frequency,
                    interval: recurrence_interval,
                    count: recurrence_count,
                    until: recurrence_until,
                    by_day: recurrence_by_day,
                });
                let reminders = reminder_minutes
                    .into_iter()
                    .map(|minutes_before| crate::caldav::EventReminder {
                        minutes_before,
                        action: Some("DISPLAY".to_string()),
                    })
                    .collect();
                let attendees = attendee
                    .into_iter()
                    .map(|email| crate::caldav::EventAttendee {
                        email,
                        ..Default::default()
                    })
                    .collect();

                commands::create_event(commands::EventInput {
                    calendar_id: calendar,
                    title,
                    start,
                    end,
                    timezone,
                    location,
                    description,
                    attendees,
                    recurrence,
                    reminders,
                })
                .await
            }
            EventsCommands::Update {
                event_id,
                calendar,
                title,
                start,
                end,
                timezone,
                location,
                description,
                attendee,
                clear_attendees,
                recurrence_freq,
                recurrence_interval,
                recurrence_count,
                recurrence_until,
                recurrence_by_day,
                clear_recurrence,
                reminder_minutes,
                clear_reminders,
            } => {
                let recurrence = recurrence_freq.map(|frequency| crate::caldav::EventRecurrence {
                    frequency,
                    interval: recurrence_interval,
                    count: recurrence_count,
                    until: recurrence_until,
                    by_day: recurrence_by_day,
                });
                let reminders = (!reminder_minutes.is_empty()).then(|| {
                    reminder_minutes
                        .into_iter()
                        .map(|minutes_before| crate::caldav::EventReminder {
                            minutes_before,
                            action: Some("DISPLAY".to_string()),
                        })
                        .collect()
                });
                let attendees = if clear_attendees {
                    Some(Vec::new())
                } else {
                    (!attendee.is_empty()).then(|| {
                        attendee
                            .into_iter()
                            .map(|email| crate::caldav::EventAttendee {
                                email,
                                ..Default::default()
                            })
                            .collect()
                    })
                };
                commands::update_event(
                    &event_id,
                    calendar.as_deref(),
                    commands::EventPatch {
                        title,
                        start,
                        end,
                        timezone,
                        location,
                        description,
                        attendees,
                        recurrence,
                        clear_recurrence,
                        reminders,
                        clear_reminders,
                    },
                )
                .await
            }
            EventsCommands::Delete {
                event_id,
                calendar,
                confirm,
                yes,
            } => {
                if !(confirm || yes) {
                    Output::<()>::error("Confirmation required: pass --confirm to delete event").print();
                    anyhow::bail!("confirmation required");
                }
                commands::delete_event(&event_id, calendar.as_deref()).await
            }
        },

        Commands::Mcp => mcp::run_server().await,
    };

    if let Err(e) = result {
        Output::<()>::error(e.to_string()).print();
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_accepts_clear_attendees_flag_for_event_updates() {
        let cli = Cli::try_parse_from([
            "fastmail-cli",
            "events",
            "update",
            "evt-1",
            "--clear-attendees",
        ])
        .unwrap();

        match cli.command {
            Commands::Events(EventsCommands::Update {
                clear_attendees, ..
            }) => {
                assert!(clear_attendees);
            }
            _ => panic!("expected events update command"),
        }
    }

    #[test]
    fn cli_rejects_clear_attendees_with_attendee_values() {
        let result = Cli::try_parse_from([
            "fastmail-cli",
            "events",
            "update",
            "evt-1",
            "--attendee",
            "person@example.com",
            "--clear-attendees",
        ]);

        match result {
            Ok(_) => panic!("expected argument conflict"),
            Err(error) => assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict),
        }
    }
}
