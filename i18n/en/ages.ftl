app-about = Show how old the people you care about are.
cmd-list = Print everyone as a table (or JSON)
cmd-add = Add a person
cmd-edit = Edit a person
cmd-remove = Remove a person
cmd-tui = Open the interactive view

col-alias = alias
col-name = name
col-age = age
col-zodiac = zodiac
col-birthday = birthday

age-short = { $years }y { $months }m
age-long = { $years }y { $months }m { $days }d
birthday-in = birthday in { $days ->
    [one] 1 day
   *[other] { $days } days
}
birthday-today = birthday today
empty-store = No people yet. Add one with: ages add <alias> --name <name> --birth <date>
added = Added { $alias }.
updated = Updated { $alias }.
removed = Removed { $alias }.
confirm-remove = Remove { $alias }? [y/N]
time-unknown = time unknown
tz-local = local time

zodiac-aries = Aries
zodiac-taurus = Taurus
zodiac-gemini = Gemini
zodiac-cancer = Cancer
zodiac-leo = Leo
zodiac-virgo = Virgo
zodiac-libra = Libra
zodiac-scorpio = Scorpio
zodiac-sagittarius = Sagittarius
zodiac-capricorn = Capricorn
zodiac-aquarius = Aquarius
zodiac-pisces = Pisces

tui-title = ages
tui-help = ↑↓ select  a add  e edit  d delete  s sort  v view  l language  q quit
tui-help-form = Tab next field  Enter save  Esc cancel
tui-help-confirm = y yes  n no
tui-help-error = press any key
tui-sort = sort: { $sort }
sort-age = age
sort-alias = alias
sort-birthday = birthday
tui-delete-title = Delete
tui-delete-body = Delete { $alias }?
tui-error-title = Error
tui-form-add = Add person
tui-form-edit = Edit person
field-alias = alias
field-first-name = first name
field-last-name = last name
field-date = birth date (YYYY-MM-DD or DD.MM.YYYY)
field-time = birth time (HH:MM, optional)
field-tz = timezone (IANA, optional)
field-avatar = avatar image path (optional)
no-avatar = no avatar

error-bad-alias = Alias "{ $alias }" is invalid: use letters, digits, - and _ only.
error-duplicate-alias = Alias "{ $alias }" already exists.
error-unknown-alias = No person with alias "{ $alias }".
error-empty-name = First name is required.
error-bad-date = "{ $value }" is not a date. Use YYYY-MM-DD or DD.MM.YYYY.
error-bad-time = "{ $value }" is not a time. Use HH:MM.
error-bad-tz = "{ $value }" is not a known timezone (e.g. Europe/Istanbul).
error-bad-image = Could not read "{ $path }" as an image.
error-store-parse = Could not parse { $path }: { $reason }
error-confirm-needs-tty = Refusing to remove without confirmation; pass --yes when not running in a terminal.
error-store-write = Could not write { $path }: { $reason }

view-calendar = calendar
view-years = years
view-months = months
view-weeks = weeks
view-days = days
view-hours = hours
view-years-value = { $years } years
view-months-value = { $months } months { $days } days
view-weeks-value = { $weeks } weeks { $days } days
view-days-value = { $days } days
view-hours-value = { $hours } hours
tui-view = view: { $view }
tui-pick-lang = Language
tui-pick-sort = Sort by
tui-pick-view = Age view
tui-help-pick = ↑↓ choose  Enter select  Esc cancel
lang-en = English
lang-tr = Türkçe
