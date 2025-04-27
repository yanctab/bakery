# Introduction

The minimal, blazing-fast, and infinitely customizable prompt for any shell! For more
information please see

    https://starship.rs/

# Bakery & Starship

To enable starship eyecandy make sure that your terminal is using a Nerd Font

    https://www.nerdfonts.com/

How to install a font and have your terminal to use it depends on what OS
you are running and what terminal. But the concept is install the font and
find the current profile to change the font for the terminal to use the Nerd
font that has been installed. Without the Nerd Font the icons will not work
but it can still be functional.

## Config

The actual starship config can be found at

   /etc/bakery/bkry-startship.toml

when the bakery deb package has been installed in the bakery project it can
be found at 

    https://github.com/yanctab/bakery/blob/main/scripts/bkry-starship.toml

The default starship config is based on

   https://starship.rs/presets/gruvbox-rainbow

### workspace.json

To run starship inside docker add the following to the workspace.json

<pre>
 "-v ${HOME}/.cache/starship:${HOME}/.cache/starship"
</pre>

This will expose the .cache/starship which starship needs to be able to
write to. Before starting a shell make sure there is a directory in your
home folder by running

<pre>
mkdir -p ~/.cache/starship
</pre>


## Eyecandy

To enter eyecandy mode start a shell

<pre>
bakery shell -c <config> --eyecandy
</pre>

Or start a shell

<pre>
bakery shell -c <config>
</pre>

and then from inside the bakery shell enter eyecandy mode by running

<pre>
eyecandy
</pre>
