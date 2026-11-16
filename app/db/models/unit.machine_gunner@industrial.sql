-- unit.machine_gunner@industrial: Machine gunner (mesh.machine_gunner_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.machine_gunner_industrial','Machine gunner');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.machine_gunner_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('loader','figure','mat.cloth',1,'','{"x":0.42,"z":-0.2,"yaw":-20,"attack":7,"hold_left":24,"hold_right":18,"legs":"#4e4a3a"}'),
  ('loader_helmet','dome','mat.olive_drab',0,'loader.head','{"y":-0.1,"w":0.27,"h":0.14,"segments":6}'),
  ('loader_helmet_brim','cylinder','mat.olive_drab',0,'loader.head','{"y":-0.1,"w":0.34,"h":0.02,"segments":6,"far":0}'),
  ('belt_box','box','mat.olive_drab',0,'loader.left_hand','{"y":-0.14,"w":0.08,"d":0.18,"h":0.13,"far":0}'),
  ('crate','box','mat.olive_drab',0,'','{"x":-0.12,"z":-0.42,"w":0.3,"d":0.26,"h":0.3}'),
  ('gunner','figure','mat.cloth',1,'','{"x":-0.12,"y":-0.08,"z":-0.42,"seat":1,"attack":7,"hold_right":72,"hold_left":72,"legs":"#4e4a3a","far":0}'),
  ('gunner_helmet','dome','mat.olive_drab',0,'gunner.head','{"y":-0.1,"w":0.27,"h":0.14,"segments":6}'),
  ('gunner_helmet_brim','cylinder','mat.olive_drab',0,'gunner.head','{"y":-0.1,"w":0.34,"h":0.02,"segments":6,"far":0}'),
  ('tripod_front','box','mat.cast_iron',0,'','{"x":-0.12,"z":0.3,"w":0.035,"d":0.035,"h":0.5,"pitch":-22}'),
  ('tripod_w','box','mat.cast_iron',0,'','{"x":-0.3,"z":-0.05,"w":0.035,"d":0.035,"h":0.5,"roll":22,"pitch":18,"far":0}'),
  ('tripod_e','box','mat.cast_iron',0,'','{"x":0.06,"z":-0.05,"w":0.035,"d":0.035,"h":0.5,"roll":-22,"pitch":18,"far":0}'),
  ('jacket','cylinder','mat.olive_drab',0,'','{"x":-0.12,"y":0.47,"z":-0.12,"w":0.11,"h":0.72,"segments":7,"pitch":88}'),
  ('receiver','box','mat.cast_iron',0,'','{"x":-0.12,"y":0.42,"z":-0.2,"w":0.12,"d":0.26,"h":0.12}'),
  ('muzzle','cylinder','mat.cast_iron',0,'jacket','{"y":0.72,"w":0.05,"h":0.08,"segments":5,"far":0}'),
  ('spark','cone','mat.flash',0,'muzzle','{"y":0.08,"w":0.09,"h":0.09,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.3,"w":0.3,"h":0.3,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.machine_gunner_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.machine_gunner'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.machine_gunner_industrial'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
