-- unit.bombard@gunpowder: Bombard (mesh.bombard_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bombard_gunpowder','Bombard');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bombard_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.oak',0,'','{"y":0.18,"w":0.62,"d":1.4,"h":0.24}'),
  ('trucks_w','wheel','mat.oak',0,'','{"x":-0.36,"z":-0.45,"w":0.34,"t":0.1,"spokes":0,"segments":8,"repeat":2,"dz":0.9}'),
  ('trucks_e','wheel','mat.oak',0,'','{"x":0.36,"z":-0.45,"w":0.34,"t":0.1,"spokes":0,"segments":8,"repeat":2,"dz":0.9}'),
  ('cradle','box','mat.oak',0,'','{"y":0.42,"z":-0.1,"w":0.5,"d":1,"h":0.12}'),
  ('barrel','cylinder','mat.iron',0,'','{"y":0.72,"z":-0.45,"w":0.48,"h":0.95,"taper":0.06,"segments":10,"pitch":84,"recoil":0.14,"tint":"#7a7e86"}'),
  ('breech','dome','mat.iron',0,'barrel','{"y":0,"w":0.5,"h":0.18,"segments":10,"pitch":180,"tint":"#6a6e76","far":0}'),
  ('hoops','cylinder','mat.iron',0,'barrel','{"y":0.25,"w":0.54,"h":0.08,"segments":10,"tint":"#5a5e66","far":0}'),
  ('hoops_2','cylinder','mat.iron',0,'barrel','{"y":0.55,"w":0.52,"h":0.08,"segments":10,"tint":"#5a5e66","far":0}'),
  ('muzzle','cylinder','mat.iron',0,'barrel','{"y":0.83,"w":0.56,"h":0.14,"segments":10,"tint":"#5a5e66"}'),
  ('bore','cylinder','mat.felt',0,'barrel','{"y":0.95,"w":0.3,"h":0.03,"segments":8,"far":0}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.92,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.7,"w":0.8,"h":0.7,"segments":8,"pitch":180,"far":0}'),
  ('lashing','box','mat.cloth',1,'','{"y":0.42,"z":0.25,"w":0.66,"d":0.16,"h":0.2}'),
  ('skirt','box','mat.cloth',1,'','{"x":-0.32,"y":0.2,"w":0.02,"d":1.2,"h":0.2,"repeat":2,"dx":0.64}'),
  ('gunner','figure','mat.cloth',1,'','{"x":-0.6,"z":-0.55,"hold_right":30,"far":0}'),
  ('linstock','cylinder','mat.timber',0,'gunner.right_hand','{"y":-0.3,"w":0.03,"h":0.85,"segments":4,"far":0}'),
  ('pennant','banner','mat.cloth',1,'','{"x":0.32,"z":-0.62,"h":1.2,"fw":0.3,"fh":0.36}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bombard_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.bombard'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.bombard_gunpowder'), (SELECT id FROM def WHERE key='mat.oak'), 100);
