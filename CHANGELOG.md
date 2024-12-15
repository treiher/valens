# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Tracking of trained muscles
- Dark mode
- Descriptions of measuring points on body fat page
- Archiving of routines
- Button for saving changes during guided training session
- Settings for hiding UI elements related to RPE and TUT
- Option to prefer exercise in training session
- Shortcut for inserting values of previous set into current set
- Suggestion of exercises that train similar muscles when replacing exercise in training session
- Splash screen

### Changed

- Use term RPE instead of intensity
- Improve display of charts when all values are zero
- Background color of sections on routine page and training session page
- Omit charts that contain no data
- Hide empty columns in training tables
- Consider sets without RPE value to be hard sets
- Default interval on exercise, body weight, body fat and menstrual cycle page to three months
- Calculate average body weight even with less than nine values
- Use 7 day centered moving total for set volume chart on training page
- Use 7 day centered moving average for RPE chart on training page
- Improve appearance of charts on training page
- Unify fonts in charts

### Fixed

- Reject trainings, body weight, body fat and period entries in the future
- Order of training sessions on training page
- Caching to improve startup time
- Set volume of training sessions by ignoring empty entries

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

[Unreleased]: https://github.com/treiher/valens/compare/v0.4.1...HEAD
[0.4.1]: https://github.com/treiher/valens/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/treiher/valens/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/treiher/valens/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/treiher/valens/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/treiher/valens/compare/1b1733763a5f904886da9d49ea545a527f11e17f...v0.1.0
