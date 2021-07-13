const stylus = require('stylus')
const pug = require('pug')
const { resolve, join } = require('path')
const fs = require('fs')
const nib = require('nib')
const fsp = fs.promises

const rootPath = resolve(__dirname, '..')
const distPath = resolve(rootPath, 'dist')
const pagesPath = resolve(rootPath, 'pages')
const publicPath = resolve(rootPath, 'public')
const stylesPath = resolve(rootPath, 'styles')

/**
 * @param {string} path
 * @param {string} ext
 */
function replaceFileExt(path, ext) {
    if (path.includes('.')) {
        return path.substring(0, path.lastIndexOf('.')) + ext
    } else {
        return path
    }
}

/** @param {string} path */
async function compilePages(path, relPath = '/') {
    const pages = await fsp.readdir(path)
    for (const page of pages) {
        const pagePath = resolve(path, page)
        const fstat = await fsp.stat(pagePath)
        if (fstat.isFile()) {
            console.log('Building Page ', relPath + page)
            const result = pug.renderFile(pagePath)
            const outputPath = join(distPath, replaceFileExt(page, '.html'))
            await fsp.mkdir(resolve(outputPath, '..'), { recursive: true })
            await fsp.writeFile(outputPath, result)
        } else if (fstat.isDirectory()) {
            compilePages(pagePath, relPath + '/' + page)
        }
    }
}

function stylusCompile(source) {
    return stylus(source)
        .set('compress', true)
        .use(nib())
        .import('nib')
        .render()
}

async function compileStyles(path, relPath = '/') {
    const styles = await fsp.readdir(path)
    for (const style of styles) {
        const stylePath = resolve(path, style)
        const fstat = await fsp.stat(stylePath)
        if (fstat.isFile()) {
            console.log('Building Style', relPath + style)
            const result = stylusCompile(await fsp.readFile(stylePath, { encoding: 'utf8' }))
            const outputPath = join(distPath, replaceFileExt(style, '.css'))
            await fsp.mkdir(resolve(outputPath, '..'), { recursive: true })
            await fsp.writeFile(outputPath, result)
        } else if (fstat.isDirectory()) {
            compilePages(stylePath, relPath + '/' + style)
        }
    }
}

async function copyPublicFiles() {
    const files = await fsp.readdir(publicPath)
    await Promise.all(files.map(v => fsp.copyFile(resolve(publicPath, v), resolve(distPath, v))))
}

async function main() {
    await fsp.rmdir(distPath, { recursive: true })
    await fsp.mkdir(distPath)
    await Promise.all([
        compilePages(pagesPath),
        compileStyles(stylesPath),
        copyPublicFiles()
    ])
}

main().catch(console.error)
