# Toast Notifications Specification

## Purpose

Global toast notification system for surfacing errors, warnings, and success messages to the user when Tauri commands fail or succeed.

## Requirements

### Requirement: Notification Store

The system SHALL provide a Svelte writable store (`notifications`) that maintains an ordered queue of notification objects. Each notification MUST contain: `id` (unique), `type` (error|warning|success|info), `title`, `message`, `timestamp`, `dismissible` (boolean). The store MUST expose `push(notification)`, `dismiss(id)`, and `clear()` methods.

#### Scenario: Push a notification

- GIVEN the notification store is empty
- WHEN `push({ type: 'error', title: 'Playback Failed', message: 'Audio device not found' })` is called
- THEN the store contains one notification with a generated unique id and current timestamp

#### Scenario: Dismiss a notification

- GIVEN the store contains 2 notifications
- WHEN `dismiss(id)` is called with the first notification's id
- THEN the store contains only the second notification

#### Scenario: Clear all notifications

- GIVEN the store contains 3 notifications
- WHEN `clear()` is called
- THEN the store is empty

### Requirement: Auto-Dismiss Timer

The system MUST auto-dismiss notifications after a configurable duration. Error notifications SHALL persist for 8 seconds; warning, success, and info notifications SHALL persist for 5 seconds. Dismissing MUST remove the notification from the store.

#### Scenario: Error auto-dismisses after 8 seconds

- GIVEN an error notification is pushed
- WHEN 8 seconds elapse without manual dismiss
- THEN the notification is removed from the store

#### Scenario: Success auto-dismisses after 5 seconds

- GIVEN a success notification is pushed
- WHEN 5 seconds elapse without manual dismiss
- THEN the notification is removed from the store

### Requirement: Maximum Visible Toasts

The system SHALL limit visible toasts to 5. When a 6th notification is pushed while 5 are visible, the OLDEST notification MUST be dismissed automatically.

#### Scenario: Overflow dismisses oldest

- GIVEN 5 notifications are visible
- WHEN a 6th notification is pushed
- THEN the oldest notification is removed and the new one is added

### Requirement: Toast Component

The system SHALL render a Toast component for each visible notification. Each toast MUST display: title (bold), message, type-colored left border, and a close button. Clicking the close button or the toast body MUST dismiss it. The toast MUST use a slide-in animation from the right on appear and slide-out on dismiss.

#### Scenario: Toast displays with type-colored border

- GIVEN an error notification exists
- WHEN the toast renders
- THEN it has a red left border, title "Playback Failed", message text, and close button

#### Scenario: Click close button dismisses toast

- GIVEN a toast is visible
- WHEN the close button is clicked
- THEN the notification is removed from the store

### Requirement: Toast Container Positioning

The ToastContainer MUST render at fixed position bottom-right, stacked vertically with newest at bottom. It SHALL use z-index above the BottomBar and ambient blur overlay.

#### Scenario: Toasts stack at bottom-right

- GIVEN 3 notifications exist
- WHEN ToastContainer renders
- THEN toasts are stacked vertically at bottom-right, newest at bottom

### Requirement: Error Wiring — Player

The player store MUST push error notifications for all Tauri command failures (play, pause, resume, next, previous, seek, setVolume). Title SHALL use i18n key `errors.PLAYBACK_ERROR`. Message SHALL include the error detail.

#### Scenario: Play command fails

- GIVEN a user clicks play
- WHEN the Tauri play command throws an error
- THEN an error toast appears with title from `errors.PLAYBACK_ERROR` and the error message

#### Scenario: Audio device unavailable

- GIVEN no audio device is available
- WHEN the user tries to play
- THEN an error toast with `errors.DEVICE_NOT_FOUND` appears

### Requirement: Error Wiring — Search

The search store MUST push an error notification when search fails. Title SHALL use i18n key `errors.SEARCH_FAILED`. The existing `searchError` writable SHALL remain for inline display.

#### Scenario: Search returns Tauri error

- GIVEN a user searches for "bohemian rhapsody"
- WHEN the Tauri search command throws an error
- THEN an error toast appears AND searchError is still set for inline display

### Requirement: Error Wiring — Favorites

The favorites store MUST push error notifications for failed add/remove/load operations. It SHALL push a success notification (type: success) when a favorite is added, using i18n key `toasts.favorite_added`.

#### Scenario: Favorite add fails

- GIVEN a user clicks "Add to Favorites"
- WHEN the Tauri addFavorite command throws
- THEN an error toast appears

#### Scenario: Favorite added successfully

- GIVEN a user clicks "Add to Favorites"
- WHEN the Tauri addFavorite command succeeds
- THEN a success toast with `toasts.favorite_added` appears

### Requirement: Error Wiring — Library

The library store MUST push error notifications for scan, load, and remove failures. It SHALL push a success notification when a scan completes, using i18n key `toasts.scan_completed`.

#### Scenario: Folder scan fails

- GIVEN a user selects a folder to scan
- WHEN the Tauri scanFolder command throws
- THEN an error toast appears

#### Scenario: Scan completes successfully

- GIVEN a user selects a folder to scan
- WHEN the scan finishes with filesAdded > 0
- THEN a success toast with `toasts.scan_completed` and file count appears

### Requirement: i18n Toast Messages

The system SHALL define i18n keys for all toast messages in both English and Spanish locales under a `toasts` namespace.

#### Scenario: English toast messages exist

- GIVEN the en.json locale file
- WHEN inspecting the `toasts` key
- THEN keys exist for: `favorite_added`, `scan_completed`, `track_added_to_queue`, `yt_dlp_missing`

#### Scenario: Spanish toast messages exist

- GIVEN the es.json locale file
- WHEN inspecting the `toasts` key
- THEN the same keys exist with Spanish translations