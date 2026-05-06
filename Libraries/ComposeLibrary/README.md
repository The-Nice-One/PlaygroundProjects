# Compose Library

[Code.org](https://code.org) App Lab library to build declarative User Interfaces.

# Motive

During my AP Computer Science Principles class I was tasked multiple times to create user interfaces (UIs) in Code.org. In many cases the visual editor sufficed. However, for complex and dynamic UIs I needed an engine more capable. As a result, I created Compose Library, a library which at its core generates HTML strings, handles event hooks, and provides some ergonomic additions. The main application for this library is shown in my submission to the AP Computer Science Create Performance Task.

# Features

* Compose Objects; Javascript objects that represent HTML elements; p(), div(), img(), etc. Nesting elements fully supported as well.
* Style objects; write CSS styles as javascript objects.
* Event hooks; bind functions to Compose objects to automatically attach events.
* Modular; Only use Compose Library for set elements you wish.
* `$` JQuery Inspired Operator; Easily select your UI elements and set/get select properties.

# Preview

This project contains an example showing a simple hyperlink element created using Compose Library. You can checkout the live example at [https://studio.code.org/projects/applab/451ee5ee-08c9-4d0b-9e05-cf99f74bed74](https://studio.code.org/projects/applab/451ee5ee-08c9-4d0b-9e05-cf99f74bed74).

![previewComposeLibrary](https://raw.githubusercontent.com/The-Nice-One/PlaygroundProjects/refs/heads/main/Libraries/ComposeLibrary/ComposeLibraryPreview.png)

# Usage

This project contains the static files of a Code.org App Lab library. To use, create a new App Lab project, press the settings button in the Toolbox, select "Manage Libraries", then under "Import library from ID" input `b42a44dc-33a2-4e6f-94ed-dbcfb99cc42b`, and press the "Add" button.

# License

As with all other projects in this playground, the license is CC BY-NC.