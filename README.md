# memchached

## Description
Implements a subset of a memcached server. Supports Get and Set commands, ignores Set flags but expects numbers.

Currently "working" tcp server on port 4000.

## Features

Uses tower for the tcp server and nom for parsing requests.
