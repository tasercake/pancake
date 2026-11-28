# Stackroom URL state

The Stackroom server serves the checked-in browser bundle. To change its typed
query schema, use Node.js 24 and run these commands from this directory.

```sh
npm ci
npm run build
npm test
```

Commit the source and generated bundle together. `npm run check` verifies
strict TypeScript and checks that the served asset matches the source.
