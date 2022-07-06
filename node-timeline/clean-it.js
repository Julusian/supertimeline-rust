const supertl = require('../supertimeline-napi')
const fs = require('fs')
const _ = require('underscore')


const args = process.argv
if (args.length < 4) {
    console.log("Usage: dump.json 10")
    process.exit()
}

const filename = args[2]
const filename2 = args[3]

const tl = JSON.parse(fs.readFileSync(filename).toString())

function clean(obj) {
    // this is to make the test fair, as rust doesnt handle the content at all
    // obj.content = {}
    if (obj.enable && !Array.isArray(obj.enable)) {
        obj.enable = [obj.enable]
    }
    if (obj.enable) {
        for (const en of obj.enable) {
            if (typeof en.start === 'number') {
                en.start = Math.floor(en.start)
            }
        }
    }
    if (obj.priority === undefined) obj.priority = 0
    obj.priority = Math.floor(1000 * obj.priority)
    if (obj.children) {
        for (const ch of obj.children) {
            clean(ch)
        }
    }
    if (obj.keyframes) {
        for (const kf of obj.keyframes) {
            clean(kf)
        }
    }
}
for (const obj of tl) {
    clean(obj)
}


fs.writeFileSync(filename2, JSON.stringify(tl))