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

After pulling the project, run the `get` command with your AniList username.

```console
$ waifu-calendar get Owldown
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
$ waifu-calendar ics -o birthdays.ics Owldown
```

For all commands and options, use the `help` command.

```console
$ waifu-calendar help
```

## License

Copyright © 2024 Rosa Richter

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
