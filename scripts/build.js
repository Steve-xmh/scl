const stylus = require('stylus')
const pug = require('pug')
const { resolve, join, basename, extname } = require('path')
const fs = require('fs')
const nib = require('nib')
const sharp = require('sharp')
const ClosureCompiler = require('google-closure-compiler').compiler
const fsp = fs.promises

const rootPath = resolve(__dirname, '..')
const distPath = resolve(rootPath, 'dist')
const pagesPath = resolve(rootPath, 'pages')
const publicPath = resolve(rootPath, 'public')
const stylesPath = resolve(rootPath, 'styles')

const pugOptions = {
    baseUrl: process.env.BASE_URL || ''
}

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
            const result = pug.compileFile(pagePath)(pugOptions)
            const outputPath = join(distPath, replaceFileExt(page, '.html'))
            await fsp.mkdir(resolve(outputPath, '..'), { recursive: true })
            await fsp.writeFile(outputPath, result)
        } else if (fstat.isDirectory()) {
            compilePages(pagePath, relPath + '/' + page + '/')
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
            compilePages(stylePath, relPath + '/' + style + '/')
        }
    }
}

function closureCompile(input, output) {
    return new Promise((resolve, reject) => {
        const closureCompiler = new ClosureCompiler({
            js: input,
            js_output_file: output,
            compilation_level: 'ADVANCED',
            language_out: 'ECMASCRIPT5_STRICT'
        })
        closureCompiler.run((exitCode, stdOut, stdErr) => {
            if (exitCode != 0) {
                reject(stdErr)
            } else {
                resolve([stdOut, stdErr])
            }
        })
    })
}

async function copyPublicFiles(path, relPath = '/') {
    const files = await fsp.readdir(publicPath)
    const threads = []
    for (const file of files) {
        const filePath = resolve(path, file)
        const fstat = await fsp.stat(filePath)
        threads.push((async () => {
            if (fstat.isFile()) {
                switch (extname(file)) {
                    case '.png':
                    case '.jpg':
                    case '.jpeg':
                    case '.webp':
                        {
                            console.log('Processing file', relPath + file)
                            const outputPath = join(distPath, file)
                            const minOutputPath = join(distPath, basename(file, extname(file)) + '.min' + extname(file))
                            await fsp.mkdir(resolve(outputPath, '..'), { recursive: true })
                            switch (extname(outputPath)) {
                                case '.jpg':
                                case '.jpeg':
                                    {
                                        await sharp(filePath)
                                            .jpeg({
                                                progressive: true
                                            })
                                            .toFile(outputPath)
                                        await sharp(outputPath)
                                            .jpeg({
                                                progressive: true
                                            })
                                            .resize(64)
                                            .toFile(minOutputPath)
                                        break
                                    }
                                case '.png':
                                    {
                                        await sharp(filePath)
                                            .png({
                                                progressive: true
                                            })
                                            .toFile(outputPath)
                                        await sharp(outputPath)
                                            .png({
                                                progressive: true
                                            })
                                            .resize(64)
                                            .toFile(minOutputPath)
                                        break
                                    }
                                case '.webp':
                                    {
                                        await sharp(filePath)
                                            .webp({
                                                nearLossless: true
                                            })
                                            .toFile(outputPath)
                                        await sharp(outputPath)
                                            .webp({
                                                nearLossless: true
                                            })
                                            .resize(64)
                                            .toFile(minOutputPath)
                                        break
                                    }
                            }
                            break
                        }
                    case '.json':
                        {
                            console.log('Minifing json file', relPath + file)
                            const outputPath = join(distPath, file)
                            await fsp.mkdir(resolve(outputPath, '..'), { recursive: true })
                            const rawJson = await fsp.readFile(filePath, { encoding: 'utf8' })
                            await fsp.writeFile(outputPath, JSON.stringify(JSON.parse(rawJson)))
                            break
                        }
                    case '.js':
                        {

                            console.log('Minifing script', relPath + file)
                            const outputPath = join(distPath, file)
                            await fsp.mkdir(resolve(outputPath, '..'), { recursive: true })
                            const [, err] = await closureCompile(filePath, outputPath)
                            if (err) {
                                console.log('Minified script with warns', relPath + file, err)
                            }
                            break
                        }
                    default:
                        {
                            console.log('Copying file', relPath + file)
                            const outputPath = join(distPath, file)
                            await fsp.mkdir(resolve(outputPath, '..'), { recursive: true })
                            await fsp.copyFile(filePath, outputPath)
                        }
                }
            } else if (fstat.isDirectory()) {
                copyPublicFiles(filePath, relPath + '/' + file)
            }
        })())
    }
    await Promise.all(threads)
}

async function main() {
    await fsp.rmdir(distPath, { recursive: true })
    await fsp.mkdir(distPath)
    await Promise.all([
        compilePages(pagesPath),
        compileStyles(stylesPath),
        copyPublicFiles(publicPath)
    ])
}

main().catch(console.error)
