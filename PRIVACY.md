# Privacy Policy for Big Screen Launcher

Last updated: 2026-04-25

Big Screen Launcher is a local-first Windows game launcher. This Privacy Policy explains what data the application reads, stores, and transmits when you use it.

## 1. Summary

- Big Screen Launcher does not require you to create an account.
- Big Screen Launcher does not sell personal data.
- Big Screen Launcher does not include advertising.
- Big Screen Launcher primarily reads game installation and achievement data that already exists on your device.
- Big Screen Launcher may connect to Steam-operated endpoints to download artwork and fetch public global achievement percentages for Steam games.

## 2. Data the App Accesses on Your Device

To build your local library and show related details, the app may read information already stored on your PC, including:

- Steam installation metadata, library metadata, and local achievement-related files.
- Epic Games Launcher manifest files and local launcher settings used to determine installed games and last played timestamps.
- Xbox / Microsoft Store installation metadata available on the local system.
- Windows registry values required to enable or disable launch on startup.
- Game executable paths, install directories, local icon resources, and related local metadata needed to show your library.

This access is used to detect installed games, display achievement information, support launch behavior, and present artwork and icons.

## 3. Data Stored Locally by Big Screen Launcher

Big Screen Launcher stores its own local data in the current user's local application data folder, typically under LocalAppData/Big Screen Launcher/. Depending on the features you use, this may include:

- App settings in LocalAppData/Big Screen Launcher/config/settings.ini.
- Last played timestamps recorded by the launcher in LocalAppData/Big Screen Launcher/config/game_last_played.json.
- Cached achievement summaries and cached global achievement percentages in LocalAppData/Big Screen Launcher/caches/achievement_cache/.
- Cached cover art, logos, game icons, achievement icons, and DLSS lookup results in LocalAppData/Big Screen Launcher/caches/.
- Local log files such as LocalAppData/Big Screen Launcher/logs/scan_timings.log and achievement diagnostics logs when those features are triggered.

This data remains on your device unless you remove it yourself.

## 4. Network Requests

Big Screen Launcher is designed to work mainly with local data, but some features can make outbound network requests:

- Steam artwork downloads: the app may download Steam game hero images, logos, and achievement icons from Steam CDN endpoints.
- Steam public achievement statistics: the app may request public global achievement percentages from the Steam Web API endpoint for a game App ID.

These requests are used to display artwork and public community achievement percentages. The app does not currently require a Big Screen Launcher account or upload your personal profile data to operate these features.

As with any direct network request, the service you connect to may receive technical information such as your IP address, request headers, and the requested resource path.

## 5. Data Not Intentionally Collected by the App

Based on the current application behavior, Big Screen Launcher does not intentionally collect or transmit:

- Your name, email address, or phone number.
- Payment information.
- Precise location data.
- Contacts, microphone input, camera input, or uploaded files.
- A cloud copy of your local game library.

## 6. Startup Integration

If you enable launch on startup, Big Screen Launcher registers itself to start after sign-in using the mechanism supported by the current installation type. Traditional desktop installs use a Windows startup entry under the current user profile, while packaged Microsoft Store / MSIX installs use the package startup task mechanism. If you disable that setting, the app removes or disables the corresponding startup registration.

## 7. Your Choices

You can limit or remove data use by:

- Disabling game source detection options in the app settings.
- Disabling launch on startup in the app settings.
- Deleting the app's local config, caches, and logs folders under LocalAppData/Big Screen Launcher/.
- Blocking the app's network access with system or firewall controls if you do not want artwork or global Steam achievement percentages to be fetched.

Please note that removing cached data may cause the app to rebuild its library or re-download artwork the next time those features are used.

## 8. Data Retention

Local settings, caches, and logs are retained on your device until they are replaced, refreshed, or deleted by you or by normal app behavior.

## 9. Children's Privacy

Big Screen Launcher is general-purpose software and is not directed specifically to children. The app does not knowingly collect personal information from children.

## 10. Third-Party Services

Some functionality depends on data or resources provided by third-party platforms, especially Steam. Your use of those services is also subject to the privacy terms and policies of the relevant third party.

## 11. Changes to This Policy

This Privacy Policy may be updated as the app changes. The latest version should be distributed with the project source or release materials.

## 12. Contact

For privacy questions about Big Screen Launcher, contact the project maintainer through the project's public support or distribution channel.