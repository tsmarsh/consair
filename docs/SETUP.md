# GitHub Pages Setup Guide

## What's Been Added

1. **Codecov Badge** - Added to README.md showing test coverage status
2. **Product Landing Page** - Beautiful dark-themed page at `docs/index.html`
3. **GitHub Pages Workflow** - Automatic deployment on push to main/master

## Features of the Landing Page

- 🎨 Dark theme matching GitHub's design
- 📱 Responsive design (mobile-friendly)
- 🚀 Hero section with CTAs
- 📊 Feature showcase grid
- 💻 Live code examples
- 📦 Installation instructions
- 🔗 Badges and links to CI, coverage, releases

## To Enable GitHub Pages

1. Go to your repository on GitHub
2. Click **Settings**
3. In the left sidebar, click **Pages**
4. Under "Build and deployment":
   - Source: Select **GitHub Actions**
5. Click **Save**

That's it! Your site will be available at:
`https://tsmarsh.github.io/consair/`

## Testing Locally

To preview the page locally, you can use any static file server:

```bash
# Using Python
cd docs
python3 -m http.server 8000

# Using Node's http-server
npx http-server docs

# Using Rust's basic-http-server
cargo install basic-http-server
basic-http-server docs
```

Then open http://localhost:8000 in your browser.

## Codecov Setup

The codecov badge is already in the README, but you'll need to:

1. Sign up at https://codecov.io with your GitHub account
2. Add your repository to Codecov
3. Get your CODECOV_TOKEN from the Codecov dashboard
4. Add it to your GitHub repository secrets:
   - Go to Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `CODECOV_TOKEN`
   - Value: (paste your token from Codecov)

Once set up, coverage reports will be automatically generated and uploaded on every CI run.

## Customization

The landing page is a single HTML file at `docs/index.html`. You can edit it to:
- Change colors (CSS variables at the top)
- Add more examples
- Update feature descriptions
- Add screenshots or diagrams

All changes pushed to main/master will automatically deploy via the GitHub Actions workflow.
