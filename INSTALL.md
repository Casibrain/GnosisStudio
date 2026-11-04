# To install a dev environment

Install Node.js:
https://nodejs.org/en/download/

Install pnpm:
```
npm install -g pnpm
```
or via https://pnpm.io/installation

Then, in command line, call:
```
pnpm install
```
and on subsequent iterations:
```
pnpm install --frozen-lockfile
```

# To run

In command line, call:

```
pnpm run electron:start
```

On your first time running, the app will take a long time to start as it's installing Python. You should get updates on this in the terminal.


# To create installer

In command line, run this ON WINDOWS: 

```
pnpm run build:complete
```

or run this ON MACOS: 

```
pnpm run electron:make
```

This will run the installer and package the app to the "out" folder. Installer will be in out/releases
