#!/usr/bin/sh

set -e

if [ ! -z "${BROWSER_WINDOW_REGEXP}" ]; then
    CURRENT_WID=$(xdotool getwindowfocus)
    WID=$(xdotool search --name "${BROWSER_WINDOW_REGEXP}" | head -1)

    if [ ! -z "${WID}" ]; then
        xdotool windowactivate "${WID}"
        xdotool key F5
        xdotool windowactivate "${CURRENT_WID}"
    else
        echo "Can't automatically reload webpage"
        echo "- Couldn't find a window ID matching '${BROWSER_WINDOW_REGEXP}'"
        echo "  It can be set in config.mk"
    fi
else
    echo "Can't automatically reload webpage"
    echo "- BROWSER_WINDOW_REGEXP environment variable not set."
    echo "  It can be set in config.mk"
fi
