const { Resolver } = require('superfly-timeline')
const fs = require('fs')
const _ = require('underscore')

const args = process.argv
if (args.length < 4) {
    console.log("Usage: dump.json 10")
    process.exit()
}

const filename = args[2]
const iterations = Number(args[3])

const tl = JSON.parse(fs.readFileSync(filename).toString())

function clean(obj) {
	obj.content = {}
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

let resolved
const times = []

for(let i = 0; i < iterations; i++) {
    const start = Date.now()

    resolved = Resolver.resolveTimeline(tl, { time: 1597158621470 })
    // const states = Resolver.resolveAllStates(resolved)

    // const state = Resolver.getState(states, 1597158621470 + 5000)

    const end = Date.now()
    times.push(end - start)
}

const sum = times.reduce((p, v) => p + v, 0)
console.log(`Completed ${times.length} iterations in ${sum}ms, averaging ${sum / times.length}ms`)

delete resolved.statistics

resolved.objects = _.sortBy(Object.entries(resolved.objects), e => e[0])
resolved.layers = _.sortBy(Object.entries(resolved.layers), e => e[0])
resolved.classes = _.sortBy(Object.entries(resolved.classes), e => e[0])

function tidyCaps(caps) {
	return _.sortBy(caps.map(c => ({
		...c,
		start: Math.ceil(c.start),
		end: ceil(c.end),
	})), c => c.id)
}

function ceil(v) {
	return typeof v === 'number' ? Math.ceil(v) : v
}

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


for (const i in resolved.objects) {
	const entry = resolved.objects[i]
	const old = entry[1]

	entry[1] = {
		resolved: {
			is_self_referencing: old.resolved.isSelfReferencing,
			instances: old.resolved.instances.map(inst => ({
				id: updateId(inst.id),
				is_first: inst.isFirst ?? false,
				start: ceil(inst.start),
				end: ceil(inst.end),
				// TODO - why are these original_* different? I don't think it matters though, as there is no change
				original_start: old.resolved.instances.length === 1 && inst.originalStart === inst.start ? null : ceil(inst.originalStart ?? null),
				original_end: old.resolved.instances.length === 1 && inst.originalEnd === inst.end ? null :ceil(inst.originalEnd ?? null),
				references: inst.references.sort(),
				caps: tidyCaps(inst.caps ?? []),
				from_instance_id: old.fromInstanceId ?? null,
				// raw: inst,
			})),
			direct_references: Array.from(new Set(old.resolved.directReferences)).sort(),
		},
		info: {
			id: old.id,
			enable: (Array.isArray(old.enable) ? old.enable : [old.enable]).map(e => ({...e, start: ceil(e.start)})),
			priority: Math.floor((old.priority ?? 0) * 1000),
			disabled: old.disabled ?? false,
			layer: old.layer,
			depth: old.resolved.levelDeep,
			parent_id: old.resolved.parentId ?? null,
			is_keyframe: old.resolved.isKeyframe ?? false,
		},
		// raw: old,
	}
}
for (const obj of resolved.objects) {
	for (const inst of obj[1].resolved.instances) {
		inst.references = inst.references.map(updateId).sort()
	}	
}

fs.writeFileSync("it-ran.json", JSON.stringify(resolved, undefined, 4))