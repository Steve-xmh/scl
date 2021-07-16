const express = require('express')
const stylus = require('stylus')
const pug = require('pug')
const { resolve, join } = require('path')
const fs = require('fs')
const nib = require('nib')
const app = express()

const rootPath = resolve(__dirname, '..')
const layoutPath = resolve(rootPath, 'layout/layout.pug')
const pagesPath = resolve(rootPath, 'pages')
const publicPath = resolve(rootPath, 'public')
const stylesPath = resolve(rootPath, 'styles')

const pugOptions = {
    baseUrl: process.env.BASE_URL || ''
}

/**
 * @param {string} path
 */
function getFileExt(path) {
    if (path.includes('.')) {
        return path.substring(path.lastIndexOf('.'))
    } else {
        return ''
    }
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

app.get('*', async (req, res, next) => {
    let path = req.path
    path = path === '/' ? '/index.html' : path
    switch (getFileExt(path)) {
        case '.html':
            {
                const relPath = join(pagesPath, replaceFileExt(path, '.pug'))
                if (fs.existsSync(relPath) && (await fs.promises.stat(relPath)).isFile()) {
                    const page = pug.compileFile(relPath)
                    res.type('html').send(page(pugOptions))
                } else {
                    next()
                }
                break
            }
        case '.css':
            {
                const relPath = join(stylesPath, replaceFileExt(path, '.styl'))
                if (fs.existsSync(relPath) && (await fs.promises.stat(relPath)).isFile()) {
                    const cssData = stylus(await fs.promises.readFile(relPath, { encoding: 'utf8' }))
                        .use(nib())
                        .import('nib')
                        .render()
                    res.type('css').send(cssData)
                } else {
                    next()
                }
                break
            }
        default:
            {
                const relPath = join(publicPath, path)
                const ext = getFileExt(relPath)
                if (fs.existsSync(relPath) && (await fs.promises.stat(relPath)).isFile()) {
                    res.type(ext.substring(1)).send(await fs.promises.readFile(relPath))
                } else {
                    next()
                }
            }
    }
})

app.use((_req, res) => {
    const notFountPath = join(pagesPath, '404.pug')
    res.type('html').send(pug.compileFile(notFountPath)(pugOptions)).end()
})

app.listen(8080, () => {
    console.log('Start listening on port 8080')
})
