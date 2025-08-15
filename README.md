# wb2kk

[![CI](https://github.com/fikovnik/wb2kk/actions/workflows/ci.yml/badge.svg)](https://github.com/fikovnik/wb2kk/actions/workflows/ci.yml)

Convert a [Wallabag](https://wallabag.it/) export (JSON) into [Karakeep](https://github.com/karakeep/karakeep) bookmarks JSON.

- Input: Wallabag export JSON (array of items) — from file or stdin
- Output: Karakeep JSON object with a "bookmarks" array
- Extras: Add custom tags to every bookmark

## Usage

```sh
wb2kk <input> [output] [options]
```

- input: Path to Wallabag export JSON file, or "-" for stdin
- output: Optional path for output JSON (defaults to stdout)

Options:

- `-t, --tag <TAG>` Additional tag to add to every bookmark (repeatable)
- `-h, --help` Print help
- `-V, --version` Print version

Examples:

- Convert a file:

  ```sh
  wb2kk wallabag.json karakeep.json
  ```

- Stream from stdin to stdout:

  ```sh
  wb2kk - < wallabag.json > karakeep.json
  ```

- Add tags:

  ```sh
  wb2kk -t wallabag -t imported wallabag.json karakeep.json
  ```

- Chain with other tools:

  ```sh
  curl -sS <https://example/export.json> | wb2kk -t imported - > out.json
  ```
