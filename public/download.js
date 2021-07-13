function h(tag, attr = {}, children = []) {
    const el = document.createElement(tag)
    for (const key in attr) {
        el[key] = attr[key]
    }
    for (const child of children) {
        el.appendChild(child)
    }
    return el
}
(async () => {
    const files = await (await fetch('/demo-download-sources.json')).json()
    const filesEl = document.querySelector('.demo-files')
    filesEl.innerHTML = ''
    for (const file of files) {
        filesEl.append(h('div', {}, [
            h('div', { className: 'name', innerText: `${file.name}` }),
            h('div', { className: 'hash', innerText: `SHA256: ${file.sha256}` }),
            h('a', { className: 'button vt', innerText: `VirusTotal 查毒报告`, href: file.virustotal }),
            ...file.sources.map(source => h('a', {
                className: 'button d',
                href: source.url,
                innerText: `从 ${source.name} 下载`
            }))
        ]))
    }
})()