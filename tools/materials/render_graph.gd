# Render a graph through Material Maker's own exporter without its editor window.
# The Windows 1.4 editor CLI stalls during startup; this SceneTree uses its autoloads directly.
extends SceneTree

func _initialize():
	call_deferred("render_graph")

func render_graph():
	var args = OS.get_cmdline_user_args()
	if args.size() != 3:
		push_error("Expected graph path, output directory and texture size")
		quit(1)
		return
	# The renderer creates its RenderingDevice asynchronously during autoload startup.
	await create_timer(2.0).timeout
	var gen = await root.get_node("mm_loader").load_gen(args[0])
	print("Graph loaded")
	if gen == null:
		push_error("Cannot load Material Maker graph: " + args[0])
		quit(1)
		return
	root.add_child(gen)
	print("Graph attached")
	# Material nodes compile their initial preview shaders asynchronously when attached.
	await create_timer(2.0).timeout
	var exported = false
	for child in gen.get_children():
		if child.has_method("export_material"):
			print("Preparing exporter")
			# This also loads the exporter profiles before export_material uses their cache.
			if child.get_export_profiles().find("Godot/Godot 4 Standard") == -1:
				push_error("Material Maker has no Godot 4 Standard export profile")
				quit(1)
				return
			var prefix = args[1].path_join(args[0].get_file().get_basename())
			print("Exporting ", prefix)
			await child.export_material(prefix, "Godot/Godot 4 Standard", int(args[2]))
			exported = true
	print("Material Maker graph exported: ", args[0])
	quit(0 if exported else 1)
