/**
 * @param {string} path
 */
function getMinUrl(path) {
    const extpos = path.lastIndexOf('.')
    return path.substring(0, extpos) + '.min' + path.substring(extpos)
}
/**
 * @param {string} url 
 * @param {Function} callback 
 * @returns {string}
 */
function fakeLoadImage(url, callback) {
    const img = document.createElement('img')
    img.onload = callback.bind(undefined, true)
    img.onerror = callback.bind(undefined, false)
    img.src = url
    return img.src
}
/**
 * @param {string} str 
 * @param {string} key 
 */
function endsWith(str, key) {
    return str.substring(str.length - key.length) === key
}
document.addEventListener('DOMContentLoaded', function () {
    if ('IntersectionObserver' in window) {
        const obs = new IntersectionObserver(function (entries) {
            entries.forEach(el => {
                if (el.intersectionRatio > 0) {
                    const target = el.target
                    const dataSrc = fakeLoadImage(target.dataset.src, function () {
                        target.src = dataSrc
                    })
                    const minSrc =  fakeLoadImage(endsWith(dataSrc, '.svg') ? dataSrc : getMinUrl(dataSrc), function (loaded) {
                        if (target.src !== dataSrc && loaded && minSrc !== dataSrc) {
                            target.src = minSrc
                        }
                    })
                    this.unobserve(target)
                }
            })
        })
        const matchesElements = document.querySelectorAll('img[data-src]')
        for (let i = 0; i < matchesElements.length; i++) {
            const el = matchesElements.item(i)
            if (el) {
                obs.observe(el)
            }
        }
    } else {
        const matchesElements = document.querySelectorAll('img[data-src]')
        for (let i = 0; i < matchesElements.length; i++) {
            const el = matchesElements.item(i)
            el.src = el.dataset.src
        }
    }
})