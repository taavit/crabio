#!/bin/bash

# Usage info
usage() {
    echo "Usage: $0 <mp3_directory>"
    echo "Streams MP3s continuously from the directory in a loop."
    echo "Press Ctrl+C to stop."
    exit 1
}

if [[ $# -ne 1 ]]; then
    echo "Error: Missing mp3 directory argument." >&2
    usage
fi

MP3_DIR="$1"

# Validate directory
if [[ ! -d "$MP3_DIR" || ! -r "$MP3_DIR" ]]; then
    echo "Error: Directory '$MP3_DIR' does not exist or is not readable." >&2
    exit 1
fi

cd "$MP3_DIR" || { echo "Failed to change directory"; exit 1; }

# Check inotify availability
HAVE_INOTIFY=false
if command -v inotifywait >/dev/null 2>&1; then
    HAVE_INOTIFY=true
else
    echo "⚠️  inotifywait not found — using polling fallback." >&2
fi

# Ensure at least one MP3 on startup
shopt -s nullglob
mp3_files=(*.mp3)
shopt -u nullglob

if [[ ${#mp3_files[@]} -eq 0 ]]; then
    echo "No MP3 files found in '$MP3_DIR'. Waiting for new ones..." >&2
fi

echo "Streaming started. Press Ctrl+C to stop."

# Global flag for clean exit
STOP=false

# Signal handler: set STOP flag and kill any stray ffmpeg
cleanup() {
    STOP=true
    echo -e "\n🛑 Stopping streaming..."
    
    # Kill *all* ffmpeg processes that stream to icecast (just in case)
    # Using pkill with strict filter avoids killing unrelated ffmpeg
    if command -v pkill >/dev/null 2>&1; then
        pkill -f "ffmpeg.*icecast" || true
    fi

    # Also try to kill the current ffmpeg PID stored in $FFMPEG_PID (if any)
    if [[ -n "$FFMPEG_PID" ]] && kill -0 "$FFMPEG_PID" 2>/dev/null; then
        echo "Terminating ffmpeg (PID: $FFMPEG_PID)..."
        kill "$FFMPEG_PID" 2>/dev/null || true
    fi

    exit 0
}

# Install trap handlers
trap cleanup SIGINT SIGTERM SIGHUP

# Track played files per cycle
declare -A played_files=()
current_playlist=()

rebuild_playlist() {
    shopt -s nullglob
    local new_files=(*.mp3)
    shopt -u nullglob
    
    # Rebuild current_playlist: preserve existing, append new
    declare -A seen=("${!played_files[@]/?/0}")  # start with already-played keys (as markers)
    current_playlist=()
    
    for f in "${new_files[@]}"; do
        if [[ -z "${seen[$f]}" ]]; then
            current_playlist+=("$f")
        fi
    done

    echo "✅ Playlist updated: ${#current_playlist[@]} file(s)"
}

rebuild_playlist

FFMPEG_PID=""

while ! $STOP; do
    if [[ ${#current_playlist[@]} -eq 0 ]]; then
        echo "⏳ Waiting for MP3 files..."
        if $HAVE_INOTIFY; then
            inotifywait -q -t 60 -e close_write,moved_to "$MP3_DIR"/*.mp3 2>/dev/null || true
        else
            sleep 5
        fi
        rebuild_playlist
        continue
    fi

    # Reset played set at end of full rotation
    if [[ ${#played_files[@]} -ge ${#current_playlist[@]} ]]; then
        echo "🔄 Full cycle complete — reshuffling playlist..."
        played_files=()
    fi

    # Shuffle and iterate current playlist
    while IFS= read -r f; do
        [[ -z "$f" ]] && continue

        if [[ -n "${played_files[$f]}" ]]; then
            continue
        fi

        echo "▶ Streaming: $f"
        played_files["$f"]=1

        # Extract metadata safely
        TITLE=$(ffprobe -v quiet -show_entries format_tags=title -of default=nw=1:nk=1 "$f" 2>/dev/null)
        ARTIST=$(ffprobe -v quiet -show_entries format_tags=artist -of default=nw=1:nk=1 "$f" 2>/dev/null)

        [[ -z "$TITLE" ]] && TITLE="${f%.mp3}"
        [[ -z "$ARTIST" ]] && ARTIST="Unknown Artist"

        echo "   Title: $TITLE | Artist: $ARTIST"

        # Launch ffmpeg in background, store PID
        ffmpeg -nostdin -re -i "$f" \
            -metadata title="$TITLE" \
            -metadata artist="$ARTIST" \
            -c copy -f mp3 \
            icecast://source:hackme@localhost:8000/radio.mp3 &
        FFMPEG_PID=$!

        # Now wait for ffmpeg to finish *or* signal to stop
        while ! $STOP && kill -0 "$FFMPEG_PID" 2>/dev/null; do
            sleep 1  # Poll every second — lightweight, allows clean exit
        done

        # If we hit STOP (e.g., Ctrl+C), kill ffmpeg explicitly
        if $STOP; then
            kill "$FFMPEG_PID" 2>/dev/null || true
            wait "$FFMPEG_PID" 2>/dev/null || true
            FFMPEG_PID=""
            break   # exit inner loop to cleanup()
        fi

        rebuild_playlist

    done < <(printf '%s\n' "${current_playlist[@]}" | shuf)

done

# Normal exit (shouldn't reach here unless STOP was set)
cleanup  # ensure cleanup runs