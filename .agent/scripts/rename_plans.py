import os
import datetime
import re
import sys

directory = "/Users/marcusmaia/Documents/Desenvolvimento/Elleven-Library/docs/plans"

print(f"Processing files in {directory}...")

for filename in os.listdir(directory):
    if not filename.endswith(".md"):
        continue

    filepath = os.path.join(directory, filename)
    
    # Get creation time (st_birthtime is for macOS)
    try:
        stat = os.stat(filepath)
        birthtime = stat.st_birthtime
    except AttributeError:
        # Fallback for systems without birthtime (Linux uses st_mtime usually as creation isn't standard, but this user is on Mac)
        birthtime = stat.st_mtime

    date_str = datetime.datetime.fromtimestamp(birthtime).strftime('%Y-%m-%d_%H:%M')
    
    # Process filename
    # Remove extension for processing
    name_no_ext = os.path.splitext(filename)[0]
    
    # Remove PLAN or PLAN- case insensitive, at start
    new_name_core = re.sub(r'^PLAN-?', '', name_no_ext, flags=re.IGNORECASE)
    
    # Remove any existing logic that looks like specific date prefix to avoid double dating if user didn't want that?
    # User instruction: "data tirada de dia de criação" and "nome original"
    # User instruction implies replacing any existing date logic? 
    # Actually, `2026-01-29_global-sorting-and-persistence.md` has a date.
    # If I add creation date, it might be `2026-01-29_09:00-2026-01-29_global...`
    # I will stick to the user's explicit instruction: Date + Original Name (minus PLAN).
    
    new_filename = f"{date_str}-{new_name_core}.md"
    new_filepath = os.path.join(directory, new_filename)

    if filename != new_filename:
        print(f"Renaming: {filename} -> {new_filename}")
        os.rename(filepath, new_filepath)
    else:
        print(f"Skipping: {filename} (Already correct)")

print("Done.")
