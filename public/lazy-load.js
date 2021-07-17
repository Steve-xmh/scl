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
 */
function fakeLoadImage(url, callback) {
    const img = document.createElement('img')
    img.onload = callback
    img.src = url
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
                    const dataSrc = el.target.dataset.src
                    const minSrc = endsWith(dataSrc, '.svg') ? dataSrc : getMinUrl(dataSrc)
                    fakeLoadImage(minSrc, function () {
                        if (minSrc !== dataSrc && el.target.src !== dataSrc) {
                            fakeLoadImage(dataSrc, function () {
                                console.log('Lazy original', dataSrc)
                                el.target.src = dataSrc
                            })
                        }
                        el.target.src = minSrc
                    })
                    this.unobserve(el.target)
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