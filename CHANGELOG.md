# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Training sections
- Definition of targets in routines
- Prediction of next menstrual cycle
- Calculation of average and variation of menstrual cycle length
- Search box to exercises and routines page
- Warning about unsaved changes before leaving page
- Possibility to create exercises while editing routines
- Loading indicator to all pages
- Charts for load and set volume
- Calendar
- Interval button for showing all values
- Display of interval bounds
- Beeps when timer expires
- Compact overview of recorded training sets
- Adjustable beep volume
- Automatic upgrade of database
- Support for Python 3.11

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

[unreleased]: https://github.com/treiher/valens/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/treiher/valens/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/treiher/valens/compare/1b1733763a5f904886da9d49ea545a527f11e17f...v0.1.0
