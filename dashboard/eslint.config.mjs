import { dirname, resolve } from "path"
import { fileURLToPath } from "url"

import { includeIgnoreFile } from "@eslint/compat"
import { FlatCompat } from "@eslint/eslintrc"

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const gitignorePath = resolve(__dirname, ".gitignore")

const compat = new FlatCompat({
  baseDirectory: __dirname,
})

const eslintConfig = [
  includeIgnoreFile(gitignorePath),
  ...compat.extends(
    "next/core-web-vitals",
    "next/typescript",
    "plugin:prettier/recommended"
  ),
  {
    rules: {
      "import/consistent-type-specifier-style": ["error", "prefer-top-level"],
      "@typescript-eslint/consistent-type-imports": "error",
      "@typescript-eslint/no-empty-object-type": "off",
      "@typescript-eslint/ban-ts-comment": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
        },
      ],
    },
  },
]

export default eslintConfig
