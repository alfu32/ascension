-- unit.howitzer@modern: Howitzer (mesh.howitzer_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.howitzer_modern','Howitzer');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.howitzer_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('gunner','figure','mat.cloth',1,'','{"x":-0.8,"z":-0.95,"yaw":20,"attack":7,"hold_right":30,"hold_left":10,"legs":"#4d4c3a","far":0}'),
  ('gunner_helmet','dome','mat.olive_drab',0,'gunner.head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6,"far":0}'),
  ('loader','figure','mat.cloth',1,'','{"x":0.8,"z":-1.15,"yaw":-25,"attack":7,"hold_right":50,"hold_left":50,"legs":"#4d4c3a","far":0}'),
  ('loader_helmet','dome','mat.olive_drab',0,'loader.head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6,"far":0}'),
  ('shell','cylinder','mat.brass',0,'loader.right_hand','{"x":0.1,"y":-0.02,"w":0.1,"h":0.34,"segments":5,"roll":90,"far":0}'),
  ('wheels','wheel','mat.rubber',0,'','{"x":-0.56,"w":0.8,"t":0.17,"spokes":1,"segments":12,"repeat":2,"dx":1.12}'),
  ('axle','box','mat.cast_iron',0,'','{"y":0.36,"w":1.06,"d":0.1,"h":0.1}'),
  ('carriage','box','mat.olive_drab',1,'','{"y":0.42,"z":-0.05,"w":0.5,"d":0.72,"h":0.22}'),
  ('trail_w','box','mat.olive_drab',0,'','{"x":-0.45,"z":-1.3,"w":0.14,"d":0.12,"h":1.3,"pitch":72,"yaw":17}'),
  ('trail_e','box','mat.olive_drab',0,'','{"x":0.45,"z":-1.3,"w":0.14,"d":0.12,"h":1.3,"pitch":72,"yaw":-17}'),
  ('spades','box','mat.cast_iron',0,'','{"x":-0.45,"z":-1.36,"w":0.24,"d":0.05,"h":0.16,"repeat":2,"dx":0.9,"far":0}'),
  ('shield','box','mat.olive_drab',0,'','{"y":0.5,"z":0.3,"w":0.9,"d":0.04,"h":0.44,"taper":0.08}'),
  ('shield_band','box','mat.olive_drab',1,'','{"y":0.8,"z":0.325,"w":0.82,"d":0.01,"h":0.12,"far":0}'),
  ('barrel','cylinder','mat.cast_iron',0,'','{"y":0.66,"z":-0.55,"w":0.13,"h":2.1,"taper":0.3,"segments":7,"pitch":58,"recoil":0.14}'),
  ('recuperator','cylinder','mat.olive_drab',0,'barrel','{"z":0.09,"w":0.12,"h":0.85,"segments":6,"far":0}'),
  ('brake','box','mat.cast_iron',0,'barrel','{"y":2,"w":0.2,"d":0.12,"h":0.18}'),
  ('spark','cone','mat.flash',0,'brake','{"y":0.18,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.7,"w":0.75,"h":0.7,"segments":8,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.howitzer_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.howitzer'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.howitzer_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
