# waifu-calendar

A tool to fetch the birthdays of your favorite anime characters.

Uses the [AniList](https://anilist.co) API.
Given an AniList username, fetches that user's favorite characters and reports on upcoming birthdays.

## Installation

Get it using `cargo`:

```console
$ cargo install waifu-calendar
```

## Usage

After pulling the project, run `waifucal get` with your AniList username.

```console
$ waifucal get Owldown
Fetching favorite character birthdays for username Owldown

Upcoming birthdays (next 30 days):

        Maki Zenin               5d January 20      2024-01-20

Future birthdays:

        Hitori Gotou            37d February 21     2024-02-21
        Homura Akemi            38d February 22     2024-02-22
        Misuzu Gundou           38d February 22     2024-02-22
        Saichi Sugimoto         46d March 1         2024-03-01
        Carol Olston            53d March 8         2024-03-08
        Miko Iino              111d May 5           2024-05-05
        Ranko Mannen           115d May 9           2024-05-09
        Kento Nanami           170d July 3          2024-07-03
        Tomo Aizawa            174d July 7          2024-07-07
        Mika                   180d July 13         2024-07-13
        Kurisu Makise          192d July 25         2024-07-25
        Nobara Kugisaki        205d August 7        2024-08-07
        Luka Urushibara        228d August 30       2024-08-30
        Hitohito Tadano        258d September 29    2024-09-29
        Madoka Kaname          262d October 3       2024-10-03
        Misato Katsuragi       328d December 8      2024-12-08
        Asirpa                 352d January 1       2025-01-01
```

You can also generate a `*.ics` file with the next year's worth of birthdays,
compatible with Google Calendar, iCal, Thunderbird, or probably whatever other calendar you use.

```console
$ waifucal ics -o birthdays.ics Owldown
```

For all commands and options, use the `help` command.

```console
$ waifu-calendar help
```

### As an HTTP Server

This project defines a server executable that presents a simple HTTP interface to access ths functionality.
To install it, enable the `http` feature when you install this tool:

```console
$ cargo install waifu-calendar --features http
```

Then run it with the name `waifu-server`

```console
$ waifu-server
```

The server will then be accessible at <http://localhost:8080>.

### As a library

If you just wish to use this project as a library, disable all default features:

```console
$ cargo add waifu-calendar --no-default-features
```

The `ics` feature for generating ICalendar files is also a default feature,
so add that feature back if you need it.

## License

Copyright © 2024 Rosa Richter

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
