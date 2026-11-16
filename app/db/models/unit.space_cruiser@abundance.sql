-- unit.space_cruiser@abundance: Space Cruiser (mesh.space_cruiser_abundance)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.space_cruiser_abundance','Space Cruiser');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.space_cruiser_abundance';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.steel',0,'','{"y":0.2,"z":-0.2,"w":1,"d":2.9,"h":0.5,"taper":0.12,"tint":"#9aa4b2"}'),
  ('hammer','box','mat.steel',1,'','{"y":0.2,"z":1.6,"w":1.55,"d":0.62,"h":0.44,"taper":0.2}'),
  ('hammer_tips','box','mat.steel',0,'','{"x":-0.7,"y":0.26,"z":1.62,"w":0.16,"d":0.66,"h":0.34,"repeat":2,"dx":1.4,"tint":"#8e97a3"}'),
  ('neck','box','mat.steel',0,'','{"y":0.2,"z":1.15,"w":0.7,"d":0.4,"h":0.4,"tint":"#8e97a3","far":0}'),
  ('deck','box','mat.steel',1,'','{"y":0.7,"z":-0.2,"w":0.34,"d":2.5,"h":0.04,"far":0}'),
  ('tower','box','mat.steel',0,'','{"y":0.7,"z":-0.3,"w":0.55,"d":0.75,"h":0.65,"taper":0.35,"tint":"#c3cbd6"}'),
  ('tower_top','box','mat.steel',1,'','{"y":1.35,"z":-0.3,"w":0.62,"d":0.36,"h":0.12}'),
  ('window','box','mat.light_strip',0,'','{"y":1.39,"z":-0.11,"w":0.42,"d":0.02,"h":0.05,"far":0}'),
  ('mast','cylinder','mat.iron',0,'','{"y":1.47,"z":-0.35,"w":0.05,"h":0.5,"segments":4,"tint":"#7c8592","far":0}'),
  ('radiators','box','mat.white_metal',0,'','{"x":-0.85,"y":0.42,"z":-0.8,"w":0.7,"d":1,"h":0.04,"repeat":2,"dx":1.7}'),
  ('radiator_ribs','box','mat.iron',0,'','{"x":-0.85,"y":0.46,"z":-0.8,"w":0.6,"d":0.05,"h":0.03,"repeat":2,"dx":1.7,"tint":"#4a5260","far":0}'),
  ('engines','cylinder','mat.iron',0,'','{"x":-0.34,"y":0.42,"z":-2,"w":0.36,"h":0.4,"segments":8,"pitch":90,"repeat":3,"dx":0.34,"tint":"#4a5260","far":0}'),
  ('drives','wheel','mat.glow',0,'','{"x":-0.34,"y":0.42,"z":-2.02,"yaw":90,"w":0.3,"t":0.05,"spokes":0,"segments":8,"repeat":3,"dx":0.34}'),
  ('fore','box','mat.steel',0,'','{"x":0,"y":0.64,"z":1.62,"w":0.4,"d":0.4,"h":0.13,"taper":0.25,"traverse":22,"tint":"#9aa4b2"}'),
  ('fore_guns','cylinder','mat.iron',0,'fore','{"x":-0.1,"y":0.06,"z":0.12,"w":0.055,"h":0.5,"segments":4,"pitch":90,"repeat":3,"dx":0.1,"tint":"#4a5260","far":0}'),
  ('fore_flash','box','mat.flash',0,'fore','{"y":0.03,"z":0.64,"w":0.36,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('fore_bolts','cylinder','mat.plasma',0,'fore_flash','{"x":-0.1,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":3,"dx":0.1,"far":0}'),
  ('mid','box','mat.steel',0,'','{"x":0,"y":0.74,"z":0.5,"w":0.4,"d":0.4,"h":0.13,"taper":0.25,"traverse":22,"tint":"#9aa4b2"}'),
  ('mid_guns','cylinder','mat.iron',0,'mid','{"x":-0.1,"y":0.06,"z":0.12,"w":0.055,"h":0.5,"segments":4,"pitch":90,"repeat":3,"dx":0.1,"tint":"#4a5260","far":0}'),
  ('mid_flash','box','mat.flash',0,'mid','{"y":0.03,"z":0.64,"w":0.36,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('mid_bolts','cylinder','mat.plasma',0,'mid_flash','{"x":-0.1,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":3,"dx":0.1,"far":0}'),
  ('aft','box','mat.steel',0,'','{"x":0,"y":0.74,"z":-1.15,"w":0.4,"d":0.4,"h":0.13,"taper":0.25,"traverse":-22,"tint":"#9aa4b2"}'),
  ('aft_guns','cylinder','mat.iron',0,'aft','{"x":-0.1,"y":0.06,"z":0.12,"w":0.055,"h":0.45,"segments":4,"pitch":90,"repeat":3,"dx":0.1,"tint":"#4a5260","far":0}'),
  ('aft_flash','box','mat.flash',0,'aft','{"y":0.03,"z":0.59,"w":0.36,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('aft_bolts','cylinder','mat.plasma',0,'aft_flash','{"x":-0.1,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":3,"dx":0.1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.space_cruiser_abundance'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.space_cruiser'), NULL, (SELECT id FROM def WHERE key='age.abundance'), (SELECT id FROM def WHERE key='mesh.space_cruiser_abundance'), (SELECT id FROM def WHERE key='mat.steel'), 100);
