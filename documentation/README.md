# kakoi - documentation source #

## dependencies ##

- GNU Make (managing documentation compilation)
- Pandoc (compiling markdown files to HTML)
- Inkscape (compiling svg files to png files)
- entr [optional] (watching source files for modification)
- xdotool [optional] (reloading browser upon source file changes)

Arch Linux:

```sh
pacman -S make inkscape pandoc entr xdotool
```

## building documentation ##

If you want your browser to automatically reload the current tab upon
documentation source file change, copy `config.mk.template` and customize
BROWSER-* related variables to your liking.

- Compile documentation and open files in a web browser, reload on changes

  ```sh
  make
  ```
- Compile documentation

  ```sh
  make compile
  ```
  
  Compiled files should be produced in the `build/` folder.

- Remove compiled files

  ```sh
  make clean
  ```
