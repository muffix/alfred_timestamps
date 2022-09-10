# Timestamp Converter

Alfred workflow to convert timestamps into dates and vice versa.

The workflow can be triggered using the `ts` keyword in Alfred or as a universal action on text.

It currently supports the following input formats:

- UNIX timestamps in seconds, millis or nanos
- ISO 8601-compatible strings
- RFC 2822-compatible strings
- `%Y-%m-%d %H:%M:%S`
- `%Y-%m-%d`
- `%H:%M:%S` (uses today's day)

Upon selection, the output is copied to the clipboard.

## Installation

1. Download the workflow from the [releases][1] page.
2. Remove the quarantine attribute by running `xattr -r -d com.apple.quarantine /path/to.alfredworkflow`
3. Double-click the workflow file to import it into Alfred

The quarantine flag is added by [Gatekeeper] which by default prevents unsigned binaries from running.

[1]: https://github.com/muffix/alfred_timestamps/releases
[Gatekeeper]: https://support.apple.com/en-us/HT202491
