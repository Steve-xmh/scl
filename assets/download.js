/**
 * @param {string} tag 
 * @param {*} attr 
 * @param {Array<*>} children 
 * @returns 
 */
function h(tag, attr = {}, children = []) {
    const el = document.createElement(tag)
    for (const key in attr) {
        el[key] = attr[key]
    }
    children.forEach(child => {
        el.appendChild(child)
    })
    return el
}
/**
 * @param {Function} callback 
 */
function getDownloadSource(callback) {
    const xhr = new XMLHttpRequest()
    xhr.addEventListener('load', function () {
        callback(JSON.parse(xhr.responseText))
    })
    xhr.open('GET', 'demo-download-sources.json')
    xhr.send()
}
/**
 * @typedef {{
 *      name: string,
 *      url: string
 * }}
 */
var DownloadSource

/**
 * @typedef {{
 *      name: string,
 *      sha256: string,
 *      virustotal: string,
 *      sources: Array<DownloadSource>
 * }}
 */
var FileItem

/**
 * @param {Array<FileItem>} files 
 */
function downloadSourceCallback(files) {
    const filesEl = document.querySelector('.demo-files')
    filesEl.innerHTML = ''
    {
        const latest = files.shift()
        const children = [
            h('div', { className: 'name', innerText: `最新版本：${latest['name']}` }),
            h('div', { className: 'hash', innerText: `SHA256: ${latest['sha256']}` }),
            h('a', { className: 'button vt', innerText: `VirusTotal 查毒报告`, href: latest['virustotal'] }),
        ]
        latest['sources'].forEach(source => children.push(h('a', {
            className: 'button d',
            href: source['url'],
            innerText: `从 ${source['name']} 下载`
        })))
        filesEl.append(h('div', { className: 'file show' }, children))
    }
    const olderVersions = files.map(file => {
        const children = [
            h('div', { className: 'name', innerText: `${file['name']}` }),
            h('div', { className: 'hash', innerText: `SHA256: ${file['sha256']}` }),
            h('a', { className: 'button vt', innerText: `VirusTotal 查毒报告`, href: file['virustotal'] }),
        ]
        file['sources'].forEach(source => children.push(h('a', {
            className: 'button d',
            href: source['url'],
            innerText: `从 ${source['name']} 下载`
        })))
        return h('div', { className: 'file' }, children)
    })
    const showAllButton = h('div', {
        className: 'showall', innerText: '显示历史版本', onclick() {
            olderVersions.forEach(e => e.classList.toggle('show'))
        }
    })
    filesEl.append(showAllButton)
    olderVersions.forEach(e => filesEl.append(e))
}
getDownloadSource(downloadSourceCallback)