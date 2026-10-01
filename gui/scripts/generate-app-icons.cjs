#!/usr/bin/env node
/**
 * App Icon Generator for AutoRename-Revived
 *
 * Generates consistent app icons from an SVG definition in icons.config.json.
 *
 * Usage:
 *   node scripts/generate-app-icons.cjs --config icons.config.json
 *
 * Outputs: PNG (multiple sizes), ICO (Windows), ICNS (macOS).
 * The config lives in `gui/`, so `outputDir` is resolved relative to the
 * config file — it points at `../src-tauri/icons`, which is where
 * `tauri.conf.json` looks for `bundle.icon`.
 */

const fs = require('fs')
const path = require('path')

// Check for sharp
let sharp
try {
  sharp = require('sharp')
} catch (e) {
  console.error('Missing dependency: sharp')
  console.error('Run: pnpm add -D sharp')
  process.exit(1)
}

// Check for png2icons (creates proper multi-size ICO and ICNS)
let png2icons
try {
  png2icons = require('png2icons')
} catch (e) {
  console.error('Missing dependency: png2icons')
  console.error('Run: pnpm add -D png2icons')
  process.exit(1)
}

// Standard sizes (includes Windows DPI scaling sizes)
const SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024]

// Standard visual parameters
const CANVAS_SIZE = 1024
const CORNER_RADIUS = 200
const ICON_SCALE = 28

/**
 * Generate path attributes based on fill vs stroke mode
 */
function getPathAttrs(strokeWidth) {
  if (strokeWidth) {
    return `fill="none" stroke="white" stroke-width="${strokeWidth}" stroke-linecap="round" stroke-linejoin="round"`
  }
  return `fill-rule="evenodd" fill="white"`
}

/**
 * Generate the main app icon SVG
 */
function generateIconSvg(config) {
  const { colors, iconPath, iconContent, iconCenter = { x: 12, y: 12 }, strokeWidth } = config

  const translateX = (CANVAS_SIZE / 2) - (iconCenter.x * ICON_SCALE)
  const translateY = (CANVAS_SIZE / 2) - (iconCenter.y * ICON_SCALE)

  let iconElement
  if (iconContent) {
    iconElement = iconContent
  } else {
    const pathAttrs = getPathAttrs(strokeWidth)
    iconElement = `<path d="${iconPath}" ${pathAttrs}/>`
  }

  return `<?xml version="1.0" encoding="UTF-8"?>
<svg width="${CANVAS_SIZE}" height="${CANVAS_SIZE}" viewBox="0 0 ${CANVAS_SIZE} ${CANVAS_SIZE}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="bgGradient" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:${colors.start}"/>
      <stop offset="100%" style="stop-color:${colors.end}"/>
    </linearGradient>
  </defs>

  <!-- Background with rounded corners and gradient -->
  <rect width="${CANVAS_SIZE}" height="${CANVAS_SIZE}" rx="${CORNER_RADIUS}" fill="url(#bgGradient)"/>

  <!-- App icon - centered -->
  <g transform="translate(${translateX}, ${translateY}) scale(${ICON_SCALE})">
    ${iconElement}
  </g>
</svg>`
}

/**
 * Generate all icons for the app
 */
async function generateIcons(config) {
  const { appName, outputDir, colors, iconPath, iconContent, iconCenter, strokeWidth } = config

  if (!outputDir) {
    throw new Error('icons.config.json must define "outputDir"')
  }

  console.log(`Generating ${appName} app icons...\n`)

  // Ensure output directory exists
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true })
  }

  const iconSvg = generateIconSvg({ colors, iconPath, iconContent, iconCenter, strokeWidth })

  // Save source SVG
  const svgPath = path.join(outputDir, 'icon-source.svg')
  fs.writeFileSync(svgPath, iconSvg)
  console.log('Created: icon-source.svg')

  // Generate PNG icons at various sizes
  console.log('\nGenerating PNG icons...')
  for (const size of SIZES) {
    const filename = size === 1024 ? 'icon.png' : `icon_${size}x${size}.png`
    const outputPath = path.join(outputDir, filename)

    await sharp(Buffer.from(iconSvg))
      .resize(size, size)
      .png()
      .toFile(outputPath)

    console.log(`  + ${filename}`)
  }

  // Tauri-specific named PNGs (copies for Tauri's expected filenames)
  console.log('\nGenerating Tauri-specific PNGs...')
  const tauriSizes = [
    { src: 'icon_32x32.png', dst: '32x32.png' },
    { src: 'icon_128x128.png', dst: '128x128.png' },
    { src: 'icon_256x256.png', dst: '128x128@2x.png' },
  ]
  for (const { src, dst } of tauriSizes) {
    fs.copyFileSync(path.join(outputDir, src), path.join(outputDir, dst))
    console.log(`  + ${dst} (from ${src})`)
  }

  const iconPngBuffer = fs.readFileSync(path.join(outputDir, 'icon.png'))

  // Generate Windows .ico
  // png2icons creates ICO with sizes: 16, 24, 32, 48, 64, 72, 96, 128, 256
  console.log('\nGenerating Windows .ico (16, 24, 32, 48, 64, 72, 96, 128, 256px)...')
  try {
    const icoBuffer = png2icons.createICO(iconPngBuffer, png2icons.BICUBIC, 0, true, true)
    if (icoBuffer) {
      fs.writeFileSync(path.join(outputDir, 'icon.ico'), icoBuffer)
      console.log('  + icon.ico')
    } else {
      throw new Error('png2icons returned no data')
    }
  } catch (e) {
    // The ICO is required by the Windows bundle, so this is fatal.
    throw new Error(`Could not generate icon.ico: ${e.message}`)
  }

  // Generate macOS .icns
  console.log('\nGenerating macOS .icns...')
  try {
    const icnsBuffer = png2icons.createICNS(iconPngBuffer, png2icons.BICUBIC, 0)
    if (icnsBuffer) {
      fs.writeFileSync(path.join(outputDir, 'icon.icns'), icnsBuffer)
      console.log('  + icon.icns')
    } else {
      console.log('  ! Failed to create icon.icns')
    }
  } catch (e) {
    console.log('  ! Could not generate .icns:', e.message)
  }

  console.log(`\nIcon generation complete!`)
  console.log(`Output directory: ${outputDir}`)
}

// CLI support
if (require.main === module) {
  const args = process.argv.slice(2)
  const configIndex = args.indexOf('--config')

  if (configIndex === -1 || !args[configIndex + 1]) {
    console.error('Usage: generate-app-icons.cjs --config <config.json>')
    process.exit(1)
  }

  const configPath = path.resolve(args[configIndex + 1])
  const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'))

  // Resolve outputDir relative to the config file, not the CWD.
  if (config.outputDir && !path.isAbsolute(config.outputDir)) {
    config.outputDir = path.resolve(path.dirname(configPath), config.outputDir)
  }

  generateIcons(config).catch((e) => {
    console.error(e.message)
    process.exit(1)
  })
}

module.exports = { generateIcons, generateIconSvg }
