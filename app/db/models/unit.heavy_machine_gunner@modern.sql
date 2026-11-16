-- unit.heavy_machine_gunner@modern: Heavy machine gunner (mesh.heavy_machine_gunner_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.heavy_machine_gunner_modern','Heavy machine gunner');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.heavy_machine_gunner_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('loader','figure','mat.cloth',1,'','{"x":0.45,"z":-0.15,"yaw":-30,"attack":7,"hold_left":26,"hold_right":20,"legs":"#4d4c3a"}'),
  ('loader_helmet','dome','mat.olive_drab',0,'loader.head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6}'),
  ('ammo_can','box','mat.olive_drab',0,'loader.left_hand','{"y":-0.13,"w":0.08,"d":0.2,"h":0.13,"far":0}'),
  ('gunner','figure','mat.cloth',1,'','{"x":-0.1,"y":-0.1,"z":-0.6,"seat":1,"attack":7,"hold_right":72,"hold_left":72,"legs":"#4d4c3a","far":0}'),
  ('gunner_helmet','dome','mat.olive_drab',0,'gunner.head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6,"far":0}'),
  ('tripod_front','box','mat.cast_iron',0,'','{"x":-0.1,"z":0.32,"w":0.035,"d":0.035,"h":0.42,"pitch":-35}'),
  ('tripod_w','box','mat.cast_iron',0,'','{"x":-0.38,"z":-0.32,"w":0.035,"d":0.035,"h":0.45,"roll":32,"pitch":28,"far":0}'),
  ('tripod_e','box','mat.cast_iron',0,'','{"x":0.18,"z":-0.32,"w":0.035,"d":0.035,"h":0.45,"roll":-32,"pitch":28,"far":0}'),
  ('receiver','box','mat.cast_iron',0,'','{"x":-0.1,"y":0.34,"z":-0.25,"w":0.15,"d":0.36,"h":0.14}'),
  ('grips','box','mat.cast_iron',0,'receiver','{"z":-0.21,"y":0.02,"w":0.16,"d":0.06,"h":0.1,"far":0}'),
  ('barrel','cylinder','mat.cast_iron',0,'','{"x":-0.1,"y":0.41,"z":-0.08,"w":0.055,"h":1,"segments":6,"pitch":90}'),
  ('sleeve','cylinder','mat.olive_drab',0,'barrel','{"w":0.1,"h":0.36,"segments":6,"far":0}'),
  ('hider','cylinder','mat.cast_iron',0,'barrel','{"y":0.92,"w":0.09,"h":0.1,"taper":-0.3,"segments":5,"far":0}'),
  ('shield','box','mat.olive_drab',0,'','{"x":-0.1,"y":0.26,"z":0.12,"w":0.44,"d":0.03,"h":0.3,"taper":0.1}'),
  ('belt','box','mat.brass',0,'receiver','{"x":0.12,"y":-0.1,"w":0.1,"d":0.06,"h":0.14,"roll":-30,"far":0}'),
  ('ammo_box','box','mat.olive_drab',0,'','{"x":0.12,"z":-0.25,"w":0.16,"d":0.26,"h":0.18}'),
  ('spark','cone','mat.flash',0,'hider','{"y":0.1,"w":0.1,"h":0.1,"segments":3,"flash":5,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.34,"w":0.34,"h":0.34,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.heavy_machine_gunner_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.heavy_machine_gunner'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.heavy_machine_gunner_modern'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
