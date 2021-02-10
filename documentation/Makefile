include config.mk

compile:
	pandoc \
		--from markdown \
		--to html \
		--output build/index.html \
		src/kakoi.md

	env BROWSER_WINDOW_REGEXP=$(BROWSER_WINDOW_REGEXP) ./reload-browser.sh
