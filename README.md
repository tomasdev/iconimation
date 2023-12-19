# iconimation
Exploratory hacking around use of a template Lottie to animate a font glyph

Don't count on anything working correctly.

## Usage

1. Find an icon font
   * Perhaps via https://fonts.google.com/icons?
1. Find the codepoint of the icon
   * If using fonts.google.com, click the icon and look for the heading "Code point" in the right hand pane
1. Obtain an icon font binary
   * `git clone git@github.com:google/material-design-icons.git` perhaps
1. Replace a placeholder in the template with an icon

    ```shell
    # Example assumes that material-design-icons is cloned sibling to current directory
    # and that current directory is the root a clone of this repo

    # I definitely need a Lottie that doesn't do anything!
    $ cargo run resources/templates/still.json 0xe86c ../material-design-icons/font/MaterialIconsOutlined-Regular.otf
    Wrote "still-e86c.json"

    # A spin perhaps?
    $ cargo run resources/templates/twirl.json 0xe86c ../material-design-icons/font/MaterialIconsOutlined-Regular.otf
    ```

1. Try it out
   * https://lottiefiles.github.io/lottie-docs/playground/json_editor/ perhaps

   ![Playground](resources/images/playground.png)