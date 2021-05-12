const fs = require('fs')
const _ = require('underscore')

const tl = require('../supertimeline-json/output.json')

const resolved = tl

delete tl.options.limit_count
delete tl.options.limit_time

resolved.objects = _.sortBy(Object.entries(resolved.objects), e => e[0])
resolved.layers = _.sortBy(Object.entries(resolved.layers), e => e[0])
resolved.classes = _.sortBy(Object.entries(resolved.classes), e => e[0])

let nextInstanceId = 0
const changedIds = new Map()

function updateId(old) {
	if (old.indexOf('@') === 0) {
		const n = changedIds.get(old)
		if (n) {
			return n
		} else {
			const k = `@changed_${nextInstanceId++}`
			changedIds.set(old, k)
			return k
		}
	} else {
		return old
	}
}

function tidyCaps(caps) {
	return _.sortBy(caps, c => c.id)
}
for (const obj of resolved.objects) {
	obj[1].resolved.direct_references.sort()
	for (const inst of obj[1].resolved.instances) {
		inst.id = updateId(inst.id)
		inst.caps = tidyCaps(inst.caps)
	}	
}
for (const obj of resolved.objects) {
	for (const inst of obj[1].resolved.instances) {
		inst.references = inst.references.map(updateId).sort()
	}	
}

fs.writeFileSync("tidied.json", JSON.stringify(resolved, undefined, 4))