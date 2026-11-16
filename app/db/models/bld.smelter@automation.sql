-- bld.smelter@automation: Future smelter (mesh.smelter_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.smelter_automation','Future smelter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.smelter_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('shop','box','mat.composite_panel',0,'','{"x":-0.55,"y":0.05,"z":-0.75,"w":2.3,"d":1.9,"h":1.5,"door":0.8,"door_h":0.9,"door_colour":"#2a2420"}'),
  ('tap_glow','box','mat.glow',0,'','{"x":-0.55,"y":0.05,"z":0.215,"w":0.6,"d":0.02,"h":0.6}'),
  ('shop_glass','box','mat.smart_glass',0,'','{"x":-0.55,"y":1.1,"z":-0.75,"w":2.32,"d":1.92,"h":0.2,"far":0}'),
  ('shop_band','box','mat.white_composite',1,'','{"x":-0.55,"y":1.55,"z":-0.75,"w":2.34,"d":1.94,"h":0.22}'),
  ('shop_roof','flat_roof','mat.white_composite',0,'','{"x":-0.55,"y":1.77,"z":-0.75,"w":2.34,"d":1.94,"t":0.04,"overhang":0}'),
  ('furnace','dome','mat.white_composite',1,'','{"x":-0.75,"y":1.81,"z":-0.75,"w":1.2,"h":0.55,"segments":12}'),
  ('electrodes','cylinder','mat.graphite',0,'','{"x":-1.05,"y":2.1,"z":-0.75,"w":0.09,"h":0.6,"segments":5,"repeat":3,"dx":0.3}'),
  ('electrode_glow','cylinder','mat.plasma',0,'','{"x":-0.75,"y":2.35,"z":-0.75,"w":0.34,"h":0.04,"segments":8,"far":0}'),
  ('duct','box','mat.white_composite',0,'','{"x":0.95,"y":1.5,"z":-0.8,"w":0.85,"d":0.22,"h":0.22}'),
  ('stack','cylinder','mat.white_composite',0,'','{"x":1.35,"y":0.05,"z":-1.25,"w":0.5,"h":4.4,"taper":0.3,"segments":10}'),
  ('stack_band','cylinder','mat.white_composite',1,'','{"x":1.35,"y":3.3,"z":-1.25,"w":0.42,"h":0.5,"segments":10}'),
  ('stack_glow','cylinder','mat.light_strip',0,'','{"x":1.35,"y":4.4,"z":-1.25,"w":0.36,"h":0.06,"segments":10,"far":0}'),
  ('stack_2','cylinder','mat.white_composite',0,'','{"x":1.35,"y":0.05,"z":-0.35,"w":0.42,"h":3.6,"taper":0.3,"segments":10}'),
  ('stack_2_band','cylinder','mat.white_composite',1,'','{"x":1.35,"y":2.7,"z":-0.35,"w":0.35,"h":0.4,"segments":10}'),
  ('stack_2_glow','cylinder','mat.light_strip',0,'','{"x":1.35,"y":3.6,"z":-0.35,"w":0.3,"h":0.06,"segments":10,"far":0}'),
  ('rails_bed','box','mat.graphite',0,'','{"x":0.3,"y":0,"z":1.35,"w":2.8,"d":0.5,"h":0.06}'),
  ('rails_glow','box','mat.light_strip',0,'','{"x":0.3,"y":0.06,"z":1.35,"w":2.8,"d":0.04,"h":0.01,"far":0}'),
  ('ladle_car','box','mat.white_composite',0,'','{"x":-0.3,"y":0.1,"z":1.35,"w":0.7,"d":0.45,"h":0.12}'),
  ('ladle','cylinder','mat.graphite',0,'','{"x":-0.3,"y":0.22,"z":1.35,"w":0.5,"h":0.42,"taper":0.15,"segments":8}'),
  ('ladle_glow','cylinder','mat.glow',0,'','{"x":-0.3,"y":0.64,"z":1.35,"w":0.44,"h":0.03,"segments":8,"far":0}'),
  ('scrap','cone','mat.rust',0,'','{"x":1.15,"y":0.05,"z":0.55,"w":1,"h":0.5,"segments":7}'),
  ('magnet_crane','box','mat.white_composite',0,'','{"x":0.95,"y":0.05,"z":0.1,"w":0.1,"d":0.1,"h":1.5,"far":0}'),
  ('magnet_jib','box','mat.white_composite',1,'','{"x":1.15,"y":1.5,"z":0.1,"w":0.7,"d":0.1,"h":0.1,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.65,"y":0.05,"z":1,"h":2.6,"fw":0.5,"fh":0.8,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.smelter_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.smelter'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.smelter_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
