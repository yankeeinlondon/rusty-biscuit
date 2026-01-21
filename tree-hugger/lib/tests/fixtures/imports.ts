// Test fixture for import extraction

// Simple named import
import { readFile } from "fs";

// Aliased import
import { readFile as read, writeFile as write } from "fs/promises";

// Namespace import
import * as path from "path";

// Default import (not captured by current query)
// import express from "express";

// Multiple imports from same source
import { join, resolve } from "path";

// Re-export (not tested here)
// export { foo } from "module";

console.log(readFile, read, write, path, join, resolve);
