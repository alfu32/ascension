-- bld.citadel@fusion: Citadel (mesh.citadel_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.citadel_fusion','Citadel');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.citadel_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":5.6,"d":5.6,"h":0.06,"steps":1}'),
  ('walls','wall','mat.composite_panel',0,'','{"y":0.06,"w":5,"d":5,"h":1,"t":0.35,"ring":1,"gap":1.2}'),
  ('wall_band','wall','mat.white_composite',1,'','{"y":0.8,"w":5,"d":5,"h":0.16,"t":0.39,"ring":1,"gap":1.2,"far":0}'),
  ('wall_glow','wall','mat.light_strip',0,'','{"y":0.62,"w":5,"d":5,"h":0.04,"t":0.4,"ring":1,"gap":1.2,"far":0}'),
  ('towers_n','cylinder','mat.white_composite',0,'','{"x":-2.3,"y":0.06,"z":-2.3,"w":1.1,"h":1.5,"segments":10,"repeat":2,"dx":4.6}'),
  ('towers_s','cylinder','mat.white_composite',0,'','{"x":-2.3,"y":0.06,"z":2.3,"w":1.1,"h":1.5,"segments":10,"repeat":2,"dx":4.6}'),
  ('tower_caps_n','dome','mat.white_composite',1,'','{"x":-2.3,"y":1.56,"z":-2.3,"w":1.1,"h":0.36,"segments":8,"repeat":2,"dx":4.6,"far":0}'),
  ('tower_caps_s','dome','mat.white_composite',1,'','{"x":-2.3,"y":1.56,"z":2.3,"w":1.1,"h":0.36,"segments":8,"repeat":2,"dx":4.6,"far":0}'),
  ('pylons_n','cylinder','mat.white_composite',0,'','{"x":-2.3,"y":1.8,"z":-2.3,"w":0.24,"h":2.4,"taper":0.55,"segments":6,"repeat":2,"dx":4.6}'),
  ('pylons_s','cylinder','mat.white_composite',0,'','{"x":-2.3,"y":1.8,"z":2.3,"w":0.24,"h":2.4,"taper":0.55,"segments":6,"repeat":2,"dx":4.6}'),
  ('collars_n','cylinder','mat.white_composite',1,'','{"x":-2.3,"y":3.4,"z":-2.3,"w":0.34,"h":0.18,"segments":6,"repeat":2,"dx":4.6,"far":0}'),
  ('collars_s','cylinder','mat.white_composite',1,'','{"x":-2.3,"y":3.4,"z":2.3,"w":0.34,"h":0.18,"segments":6,"repeat":2,"dx":4.6,"far":0}'),
  ('emitters_n','cone','mat.plasma',0,'','{"x":-2.3,"y":4.15,"z":-2.3,"w":0.42,"h":0.4,"segments":4,"repeat":2,"dx":4.6}'),
  ('emitters_n_under','cone','mat.plasma',0,'','{"x":-2.3,"y":4.15,"z":-2.3,"w":0.42,"h":0.3,"segments":4,"pitch":180,"repeat":2,"dx":4.6,"far":0}'),
  ('emitters_s','cone','mat.plasma',0,'','{"x":-2.3,"y":4.15,"z":2.3,"w":0.42,"h":0.4,"segments":4,"repeat":2,"dx":4.6}'),
  ('emitters_s_under','cone','mat.plasma',0,'','{"x":-2.3,"y":4.15,"z":2.3,"w":0.42,"h":0.3,"segments":4,"pitch":180,"repeat":2,"dx":4.6,"far":0}'),
  ('keep','cylinder','mat.composite_panel',0,'','{"y":0.06,"z":-0.3,"w":2.9,"h":1.3,"segments":16}'),
  ('keep_glass','cylinder','mat.smart_glass',0,'','{"y":0.6,"z":-0.3,"w":2.92,"h":0.22,"segments":16,"far":0}'),
  ('keep_band','cylinder','mat.white_composite',1,'','{"y":1.2,"z":-0.3,"w":2.96,"h":0.22,"segments":16}'),
  ('dome','dome','mat.white_composite',0,'','{"y":1.42,"z":-0.3,"w":2.9,"h":1.9,"segments":16}'),
  ('dome_glow','cylinder','mat.light_strip',0,'','{"y":1.42,"z":-0.3,"w":2.92,"h":0.04,"segments":16,"far":0}'),
  ('crown','cylinder','mat.white_composite',1,'','{"y":3.25,"z":-0.3,"w":0.6,"h":0.3,"segments":10,"far":0}'),
  ('spire','cylinder','mat.graphite',0,'','{"y":3.55,"z":-0.3,"w":0.12,"h":1.1,"taper":0.6,"segments":6}'),
  ('beacon','box','mat.plasma',0,'','{"y":4.65,"z":-0.3,"w":0.12,"d":0.12,"h":0.12,"far":0}'),
  ('gate_pylons','box','mat.white_composite',0,'','{"x":-0.75,"y":0.06,"z":2.5,"w":0.32,"d":0.5,"h":1.5,"repeat":2,"dx":1.5}'),
  ('gate_field','box','mat.light_strip',0,'','{"y":0.1,"z":2.5,"w":1.18,"d":0.03,"h":1,"far":0}'),
  ('gate_lintel','box','mat.white_composite',1,'','{"y":1.56,"z":2.5,"w":1.82,"d":0.5,"h":0.16}'),
  ('guns_w','dome','mat.white_composite',0,'','{"x":-2.3,"y":1.06,"z":0,"w":0.6,"h":0.3,"segments":10,"far":0}'),
  ('guns_e','dome','mat.white_composite',0,'','{"x":2.3,"y":1.06,"z":0,"w":0.6,"h":0.3,"segments":10,"far":0}'),
  ('barrels_w','cylinder','mat.graphite',0,'','{"x":-2.3,"y":1.2,"z":0,"w":0.08,"h":0.8,"segments":5,"yaw":-90,"pitch":78,"repeat":2,"dz":0.14,"far":0}'),
  ('barrels_e','cylinder','mat.graphite',0,'','{"x":2.3,"y":1.2,"z":0,"w":0.08,"h":0.8,"segments":5,"yaw":90,"pitch":78,"repeat":2,"dz":0.14,"far":0}'),
  ('pad_ring','cylinder','mat.light_strip',0,'','{"x":1.45,"y":0.06,"z":1.45,"w":1.1,"h":0.025,"segments":12,"far":0}'),
  ('pad','cylinder','mat.graphite',0,'','{"x":1.45,"y":0.06,"z":1.45,"w":1,"h":0.035,"segments":12,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":-1.5,"y":0.06,"z":1.5,"h":3.4,"fw":0.9,"fh":0.6,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.citadel_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.citadel'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.citadel_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
