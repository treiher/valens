# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Exercises page:
    - Properties of custom exercises, copied from the catalog when an exercise is added from it
    - Editing of force, mechanic, laterality, assistance, equipment and category of an exercise
- Exercise page:
    - Tags showing the properties of the exercise
    - Editing of force, mechanic, laterality, assistance, equipment and category of an exercise
- Settings dialog: Beep at the selected volume when the beep volume is changed

### Changed

- Loading screen shown while the app is starting
- Catalog exercise page: Order of the properties of an exercise, showing the muscles first

### Fixed

- App not starting when it is reloaded while offline

## [0.8.0] - 2026-08-24

### Added

- Training session-specific exercise notes
- 1RM (one-repetition maximum) calculator accessible from the navigation menu
- Drop set calculator accessible from the navigation menu
- Activity bar at the bottom of the screen for returning to an in-progress training session, shown on every page except the matching training session page
- Prevention of the screen turning off while a timer is running
- Automatic focus of the first input field in dialogs, with its value selected
- FFMI (Fat-Free Mass Index) page with a chart and interval selection
- Recording of notifications in the log
- User roles (user and admin)
- One-time login links for signing in without a passkey, valid for 24 hours
- Display of chart values at the hovered or touched date
- Schedule page for planning routines on days of the week, including rotations of routines across training days
- CLI commands to list, create, update and delete users
- CLI command to create a one-time login link for a user
- CLI options for reproducing the example data of the demo command
- Output of the user names when starting the demo
- Configuration option for setting the public URL
- Configuration option for disabling the username login
- Alembic configuration file in the container for using the Alembic CLI
- Login page: Sign-in with a passkey
- Home page:
    - Most recent FFMI
    - Planned routines for the current day with buttons to start the corresponding training sessions
- Training sessions page: 7-day min./max. RPE to RPE chart
- Training session page:
    - Option to open the 1RM calculator from the exercise options
    - Option to open the drop set calculator from the exercise options
    - Automatic scrolling that brings the active element into view when it changes
    - Optional scroll snapping that centers and enlarges the current element on the screen and restricts scrolling to one element at a time
    - Buttons for target and previous values for time-based sets
    - Activity bar at the bottom of the screen showing the elapsed time with a button to end the current training session
    - Automatic start of an in-progress training session when a session without recorded sets is opened
    - Automatic end of the in-progress training session when all sets are recorded or the end of the session is reached
- Routines page: Option to show and copy routine as text
- Routine page:
    - Option to show and copy routine as text
    - Rearrangement and removal of exercises, rests and sections by drag and drop
- Exercise page:
    - Estimated maximum reps to reps chart
    - Estimated 1RM to weight chart
- Profile dialog, accessible from the navigation menu:
    - Editing of the user's own data
    - Management of the user's own passkeys (registration, renaming, deletion)
- Administration dialog:
    - Body height of users
    - Role of users
    - Warning when demoting or deleting the last admin
    - Creation of one-time login links for users
    - Deletion of passkeys of users
    - Notice about unavailable passkey login and login links when no public URL is configured

### Changed

- Administration from a page to a dialog, allowing user management without leaving the current page
- Version information and the log from the administration page to the About dialog, accessible from the navigation menu
- Synchronization to include the data of the signed-in user, applying changes made outside the app without signing in again and signing out if the session was ended on the server
- Expiry of a sign-in to one year after signing in instead of one year after the last use
- Presentation of error messages from a dialog to a notification below the navigation bar, showing the reason with the affected action below it
- Severity of recoverable notifications from error to warning
- Wording of the message for a server that cannot be reached to "Server unreachable"
- Rounding of the timer countdown up to the next full second, so that each number is shown for a whole second
- Reporting of a server that does not answer in time as unreachable instead of as an unspecific error
- Server response times to be shorter
- Synchronization to skip the download of unchanged data
- Default theme to follow the theme of the system
- Example data of the demo mode to two realistic training logs
- Navigation bar:
    - "Log out" to "Sign out"
    - Opening of the menu via the menu button on all screen widths, keeping all menu entries reachable on wide screens
    - "Sign out" as the last menu entry
    - Reporting of a missing connection to the server from an indicator to a notification
    - Reporting of a failed synchronization from an indicator to a notification
- Login page: Sign-in flow to require entering the username instead of selecting a user
- Training sessions page:
    - RPE chart to average per-set RPE values instead of per-session averages
    - Wording of chart legend labels for set volume to be consistent with other pages
- Training session page:
    - Weight input resolution from 0.1 to 0.01 kg
    - Preservation of an in-progress training session when editing a different one
    - Dimming of inactive exercise sections to occur only while a training session is in progress
- Routines page: Prevention of the deletion of routines that are used in the schedule
- Routine page:
    - Weight input resolution from 0.1 to 0.01 kg
    - Order and wording of chart legend labels
- Exercise page:
    - Order of charts to show performance metrics before volume metrics
    - Order and wording of chart legend labels
- Muscles page: Wording of chart legend labels for set volume to be consistent with other pages
- Body weight page: Order of chart legend labels
- Body fat page: Order of chart legend labels
- Administration dialog: Restriction to admin users

### Fixed

- Outdated information related to the current date being shown when the app remains open past midnight
- Acceptance of invalid values by the server that could make the affected pages permanently unusable
- Error being reported for a change that was successfully saved on the server when storing the change on the device failed
- Sign-out being aborted instead of completed when the data on the device could not be removed
- Sign-in and sign-out being undone by requests of the previous session that were still in progress
- Incorrect background color in calendars
- Missing and distorted beeps of the timer
- Distorted beeps of the metronome
- Repetition of metronome beeps after the app was in the background
- Settings not being accessible in browsers without support for notifications
- Enlarged text in some places on iOS devices
- Unnecessary symbols on the on-screen keyboard of some devices for the timer
- App not starting after an update was confirmed
- Home page: Incorrect load value when the oldest training session was not the first one added
- Training sessions page:
    - Incorrect chart legend label
    - Ordering of training sessions in the table by creation instead of by date
    - Incorrect load chart when the oldest training session was not the first one added
- Training session page:
    - Missing numbering of exercises within compound sets for time-based sets
    - Manual metronome adjustments being reset immediately instead of persisting until the current element changes
    - Loading indicator staying visible after leaving the page while a change is being saved
    - Missing decimal separator on the on-screen keyboard of some devices for weight and RPE, and unnecessary symbols for reps and time
- Routine page:
    - Loading indicator staying visible after leaving the page while a change is being saved
    - Ordering of training sessions in the table by creation instead of by date
    - Ordering of previously used exercises by creation instead of by name
    - Missing decimal separator on the on-screen keyboard of some devices for weight and RPE
- Exercise page:
    - Changed muscles not being shown until the next synchronization
    - Ordering of training sessions in the table by creation instead of by date
    - Ordering of recorded sets by training session creation instead of by date
- Body weight page: Missing decimal separator on the on-screen keyboard of some devices for the weight
- Body fat page: Missing and incorrect chart legend labels for body weight

## [0.7.0] - 2026-05-10

### Added

- Home page: Direct access to routines, exercises and muscles
- Training session page: Numbering of exercises within compound sets
- Routines page:
    - Option to copy routine
    - Keeping of filter settings in browser history
    - Name pre-filled from search term when adding
- Routine page:
    - Option to archive routine
    - Option to copy routine
    - Option to delete routine
- Exercises page:
    - Option to copy exercise
    - Option to change exercise properties
    - Keeping of filter settings in browser history
    - Name pre-filled from search term when adding
- Exercise page:
    - Option to copy exercise
    - Option to delete exercise
- Interval controls: NOW button to show the interval from the first entry to today
- Error messages on input fields
- Error page for unexpected errors
- Synchronization indicator in navigation bar
- CLI argument for database path when creating config
- Support for Python 3.14

### Changed

- Home page: Separate sections for training and health
- Training page: Renamed to training sessions page
- Training session page:
    - View mode:
        - Horizontal alignment of reps, time, weight and RPE
    - Edit mode:
        - Compact layout
        - Possibility to select any exercise set as the next one
- Routines page: Always display archive
- Routine page: Editing via action buttons and clickable properties instead of edit mode
- Exercise page: Colors of property tags
- Charts on training-related pages: Unified labels
- Interval controls: Reordered predefined interval buttons and moved navigation buttons next to the interval bounds
- Action buttons in lists: Use menu for multiple actions
- Metronome, timer and stopwatch available on all pages

### Fixed

- Exercises page: Loss of muscle associations when renaming exercise
- Exercise page: Missing interval controls when selecting interval with no data
- Indefinite waiting when server request receives no response

### Removed

- **BREAKING**: Possibility to serve app from subdirectory
- Training session page: Links to routines, exercises and muscles pages

## [0.6.0] - 2025-09-14

### Added

- Exercise catalog
- Log view
- Limited offline mode: Access to cached data when offline
- Support for Python 3.13

### Changed

- Omit calendar and tables that contain no data on exercise page

### Removed

- Support for Python 3.9

### Fixed

- Caching of app

## [0.5.0] - 2025-01-26

### Added

- Tracking of trained muscles
- Dark mode
- Descriptions of measuring points on body fat page
- Archiving of routines
- Button for saving changes during guided training session
- Settings for hiding UI elements related to RPE and TUT
- Option to prefer exercise in training session
- Option to add same exercise in training session
- Shortcut for inserting values of previous set into current set
- Suggestion of exercises that train similar muscles when replacing exercise in training session
- Splash screen

### Changed

- Background color of sections on routine page and training session page
- Default interval on exercise, body weight, body fat and menstrual cycle page to three months
- Use term RPE instead of intensity
- Improve display of charts when all values are zero
- Omit charts that contain no data
- Hide empty columns in training tables
- Consider sets without RPE value to be hard sets
- Calculate average body weight even with less than nine values
- Use 7 day centered moving total for set volume chart on training page
- Use 7 day centered moving average for RPE chart on training page
- Improve appearance of charts on training page
- Unify fonts in charts
- Allow option to add exercise at any position in training session
- Keep search term on exercises and routines pages when going back in history
- Display recent and previous exercises separately
- Improve delete dialog by adding name or date

### Fixed

- Order of training sessions on training page
- Caching to improve startup time
- Set volume of training sessions by ignoring empty entries
- Reject trainings, body weight, body fat and period entries in the future

## [0.4.1] - 2024-05-20

### Fixed

- Search box in dialogs for adding and appending exercise to training session
- Missing rest when deferring penultimate exercise in training session
- Broken timer when returning to guided training session
- Fallacious summary when editing routine
- Error handling during database upgrade

## [0.4.0] - 2024-04-22

### Added

- Option to replace exercise in training session
- Option to defer exercise in training session
- Option to add set in training session
- Option to add exercise in training session
- Option to remove set in training session
- Option to remove exercise in training session
- Option to append exercise in training session
- Possibility to create empty training session
- Blinking of time when timer in training session is paused
- Sections when editing training session
- Calculation of average weekly change in body weight
- Display of estimated duration and total number of sets on routine page
- List of sets on exercise page
- Possibility to copy existing routines when creating new ones
- Support for notifications in Chrome for Android
- Possibility to disable notifications in settings
- Description on how to allow notifications in browser
- Caching to improve startup time
- Update mechanism
- Double beep 10 seconds before timer expires

### Changed

- **BREAKING**: Location must include trailing slash if app is being served from subdirectory (see README for sample configuration)
- Design of training session page
- Design of routine page
- List routines sorted by last use
- Disable automatic metronome by default
- Skip rests with automatic flag and no duration

## [0.3.0] - 2024-01-02

### Added

- Training sections
- Definition of targets in routines
- Prediction of next menstrual cycle
- Calculation of average and variation of menstrual cycle length
- Search box to exercises and routines page
- Renaming on exercise and routine page
- Warning about unsaved changes before leaving page
- Possibility to create exercises while editing routines
- Loading indicator to all pages
- Charts for load and set volume
- Calendar
- Interval button for showing all values
- Display of interval bounds
- Beeps when timer expires
- Automatic metronome
- Notifications when going to next section of training session (not supported by Chrome Android)
- Compact overview of recorded training sets
- Adjustable beep volume
- Automatic upgrade of database
- Support for Python 3.11 and 3.12

### Changed

- Improve performance by reducing network usage
- Improve workout page
- Display weekly totals/averages in charts on workouts page
- Limit minimum interval to one week
- Limit possible intervals by first value entry and current day
- Go to workout page after adding workout
- Rename workouts to training
- Rename workout to training session
- Move links to routines and exercises pages to training page

### Removed

- Support for Python 3.8
- Body weight from period chart

### Fixed

- Adding workout on same date as existing workout
- Missing last bar in bar chart
- Disabled save button on workout page in case of error

## [0.2.0] - 2022-11-04

### Changed

- Use client-side rendering (CSR) instead of server-side rendering (SSR)
- Enable changing port number when running local server using CLI

## [0.1.0] - 2021-10-16

### Added

- Initial version of web app

[Unreleased]: https://github.com/treiher/valens/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/treiher/valens/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/treiher/valens/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/treiher/valens/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/treiher/valens/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/treiher/valens/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/treiher/valens/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/treiher/valens/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/treiher/valens/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/treiher/valens/compare/1b1733763a5f904886da9d49ea545a527f11e17f...v0.1.0
