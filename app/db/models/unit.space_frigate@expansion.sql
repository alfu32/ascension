-- unit.space_frigate@expansion: Space Frigate (mesh.space_frigate_expansion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.space_frigate_expansion','Space Frigate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.space_frigate_expansion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.steel',0,'','{"y":0.2,"z":-0.15,"w":0.8,"d":2.4,"h":0.42,"taper":0.15,"tint":"#9aa4b2"}'),
  ('bow','box','mat.steel',0,'','{"y":0.22,"z":1.4,"w":0.72,"d":0.7,"h":0.36,"taper":0.6,"tint":"#c3cbd6"}'),
  ('deck','box','mat.steel',1,'','{"y":0.62,"z":-0.2,"w":0.3,"d":1.9,"h":0.04}'),
  ('bridge','box','mat.steel',0,'','{"y":0.62,"z":0.15,"w":0.44,"d":0.55,"h":0.3,"taper":0.3,"tint":"#c3cbd6"}'),
  ('bridge_band','box','mat.steel',1,'','{"y":0.8,"z":0.15,"w":0.4,"d":0.5,"h":0.06}'),
  ('window','box','mat.light_strip',0,'','{"y":0.84,"z":0.37,"w":0.26,"d":0.02,"h":0.04,"far":0}'),
  ('mast','cylinder','mat.iron',0,'','{"y":0.92,"z":0.1,"w":0.04,"h":0.35,"segments":4,"tint":"#7c8592","far":0}'),
  ('pylons','box','mat.steel',1,'','{"x":-0.62,"y":0.36,"z":-0.75,"w":0.5,"d":0.45,"h":0.07,"repeat":2,"dx":1.24}'),
  ('nacelles','cylinder','mat.steel',0,'','{"x":-0.95,"y":0.4,"z":-1.45,"w":0.34,"h":1.55,"segments":8,"pitch":90,"repeat":2,"dx":1.9,"tint":"#7c8592"}'),
  ('nacelle_caps','cone','mat.steel',1,'','{"x":-0.95,"y":0.4,"z":0.1,"w":0.34,"h":0.3,"segments":8,"pitch":90,"repeat":2,"dx":1.9}'),
  ('drives','wheel','mat.glow',0,'','{"x":-0.95,"y":0.4,"z":-1.47,"yaw":90,"w":0.28,"t":0.05,"spokes":0,"segments":8,"repeat":2,"dx":1.9}'),
  ('engine','cylinder','mat.iron',0,'','{"y":0.41,"z":-1.65,"w":0.4,"h":0.3,"segments":8,"pitch":90,"tint":"#4a5260"}'),
  ('drive','wheel','mat.glow',0,'','{"y":0.41,"z":-1.67,"yaw":90,"w":0.32,"t":0.05,"spokes":0,"segments":8}'),
  ('fore','box','mat.steel',0,'','{"x":0,"y":0.62,"z":0.8,"w":0.34,"d":0.36,"h":0.13,"taper":0.25,"traverse":22,"tint":"#9aa4b2"}'),
  ('fore_guns','cylinder','mat.iron',0,'fore','{"x":-0.06,"y":0.06,"z":0.108,"w":0.055,"h":0.5,"segments":4,"pitch":90,"repeat":2,"dx":0.12,"tint":"#4a5260","far":0}'),
  ('fore_flash','box','mat.flash',0,'fore','{"y":0.03,"z":0.628,"w":0.28,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('fore_bolts','cylinder','mat.plasma',0,'fore_flash','{"x":-0.06,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":2,"dx":0.12,"far":0}'),
  ('aft','box','mat.steel',0,'','{"x":0,"y":0.62,"z":-0.95,"w":0.34,"d":0.36,"h":0.13,"taper":0.25,"traverse":-22,"tint":"#9aa4b2"}'),
  ('aft_guns','cylinder','mat.iron',0,'aft','{"x":-0.06,"y":0.06,"z":0.108,"w":0.055,"h":0.45,"segments":4,"pitch":90,"repeat":2,"dx":0.12,"tint":"#4a5260","far":0}'),
  ('aft_flash','box','mat.flash',0,'aft','{"y":0.03,"z":0.578,"w":0.28,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('aft_bolts','cylinder','mat.plasma',0,'aft_flash','{"x":-0.06,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":2,"dx":0.12,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.space_frigate_expansion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.space_frigate'), NULL, (SELECT id FROM def WHERE key='age.expansion'), (SELECT id FROM def WHERE key='mesh.space_frigate_expansion'), (SELECT id FROM def WHERE key='mat.steel'), 100);
