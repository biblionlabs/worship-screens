## [0.1.1-alpha.8] - 2026-01-11

### 🚀 Features

- Implement egl gpu video process
- Replace cpu process video to gpu with gstreamer
- Play view works and thumbnail on preview window
- Add autocomplete for bible books
- Use installed books to autocomplete

### 🐛 Bug Fixes

- Normalize dependencies version
- Some improvements on trace
- Preview scale
- Sync font data with component
- Need open two times bug

### 💼 Other

- Update dependencies
- Try to solve cargo-dist problems

### 🚜 Refactor

- Move manager files into a folder
- Remove software process video option and keep egl

### ⚙️ Miscellaneous Tasks

- Init gstreamer migration
- Update cargo dist to generate installer with deps
## [0.1.1-alpha.7] - 2025-12-27

### 🚀 Features

- Use tantivy as unique database

### 🐛 Bug Fixes

- Content and verse font size to cover screen
- Improvement size of content and verse text
- Scheduled list take the current values from global

### ⚙️ Miscellaneous Tasks

- Release worship presentations version
## [0.1.1-alpha.6] - 2025-12-14

### 🚀 Features

- Hide cursor on output window view

### 🐛 Bug Fixes

- List area height on preview
- Draggable item line height auto calculated

### ⚙️ Miscellaneous Tasks

- Release worship presentations version
## [0.1.1-alpha.5] - 2025-12-13

### 🚀 Features

- Save all info for simple text
- View background animation more fast
- Add schedule playlist to have a list of daily content
- Drag to reorder schedule content

### 🐛 Bug Fixes

- Show value on slider label
- Min height for main window
- Small fixes on send texts
- Add to schedule and remove callbacks to saved text
- Correct paragraph song separation
- Song preview paragraphs clear paragraph focus
- Remove default saved text

### 🚜 Refactor

- Separate sections into a individual file

### ⚙️ Miscellaneous Tasks

- Remove renderable list from song list
- Release worship presentations version
## [0.1.1-alpha.4] - 2025-12-07

### 🚀 Features

- Preserve preview state on close
- Add system font selector

### 🐛 Bug Fixes

- Clear verse on clear screen
- Remove part from verse to send into preview and output
- On/off not clear contents
- Use custom font size and preview output real
- Focus on navigate with arrow keys

### ⚙️ Miscellaneous Tasks

- Separate slider font size into a components
- Separe sections on font edit
- Release worship presentations version
## [0.1.1-alpha.3] - 2025-12-07

### 🚀 Features

- Add tracing logger
- Show changelog popup when update version

### 🐛 Bug Fixes

- Crash on file watch changes
- Send to view focus and keyboard navigation

### 🚜 Refactor

- Move all dialog to base dialog component

### ⚙️ Miscellaneous Tasks

- Release worship presentations version
## [0.1.1-alpha.2] - 2025-12-06

### 🚀 Features

- Add color picker
- Implement clear state
- Play mp4 works
- Create input check component
- Improvement and fix color component
- Improvement image view rendering
- Struct to manage user data
- Renderable base component
- Video render
- Settings bible view
- Settings and user data management
- Implement install bible from settings window
- Download bible from settings works
- Add verse search
- Add color-picker show hex
- Add songs managed and searchable
- Manage removed song files event
- Set manage favorite texts
- Add support to handle media as view content
- Manage preview and output as individual video thread handled
- Edit mode for multimedia
- Easy setting logo
- Save and load last screen used
- Bible manager and realtime search
- Add enable option to renderable
- Advanced media selector (font size, colors and stroke edit)
- Move font edit into a component and use into main screen
- Use local text for raw text input
- All renderable lists focusable and navigable with keyboard
- Add button to stop play video
- Add notifications
- Add update check and notify
- Show button if need update

### 🐛 Bug Fixes

- Gradient button touch area
- Remove padding top on view
- Add more interaction fixes on renderable and use on screen tabs
- Max chars split verse into parts
- Macro to generate implementation for setting structs
- Solve double dot on file saved
- Solve some issues with media management
- Update tmp save and load
- Preserve content on change media
- On/off view window toggle button
- Ui improvement
- Reset add media preview data
- Remove preview on settings window
- Search input width
- Correct verse show
- Use button color picker instead color picker on main screen
- Sync color picker popup with selected color
- Compile in windows/mac

### 💼 Other

- Add missing dependencies
- Support setup crate
- Use remote setup core

### 🚜 Refactor

- Replace TextEdit by LineEdit to search input
- Add tmp field into view data to handle show/hide remove button
- Easy reusable show image function

### ⚙️ Miscellaneous Tasks

- Separate color slider into own component
- Update cargo dist
- Main window min width
- Clip view component
- Update deps
- Release worship presentations version
## [0.1.1-alpha.1] - 2025-11-04

### 🚀 Features

- Update base ui
- Ui advances
- Implement the screen management
- Window view fullscreen works
- Text cover all available space
- Move to view component
- Add ccargo-dist dependency
- Implement preview and send to view
- Setup settings events

### 🐛 Bug Fixes

- Songs ui bad height
- View text expand all monitor size

### 💼 Other

- Add image procesing and system open file
- Remove nasm feature and update versions

### ⚙️ Miscellaneous Tasks

- Init project
- On start event callback
- Close settings when principal window is closed too
- Setup release workflow
- Update version
- Start changelog
- Release worship presentations version {{version}}
