-- bld.smelter@gunpowder: Early modern smelter (mesh.smelter_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.smelter_gunpowder','Early modern smelter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.smelter_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1,"tint":"#a89a88"}'),
  ('furnace','box','mat.ashlar',0,'','{"x":-0.45,"z":-0.65,"w":1.6,"d":1.6,"h":1.4,"taper":0.2}'),
  ('stack','box','mat.brick',0,'','{"x":-0.45,"y":1.4,"z":-0.65,"w":0.85,"d":0.85,"h":2.3,"taper":0.3}'),
  ('stack_lip','box','mat.ashlar',0,'','{"x":-0.45,"y":3.7,"z":-0.65,"w":0.66,"d":0.66,"h":0.12}'),
  ('stack_glow','box','mat.glow',0,'','{"x":-0.45,"y":3.76,"z":-0.65,"w":0.46,"d":0.46,"h":0.1}'),
  ('tap','arch','mat.ashlar',0,'','{"x":-0.45,"z":0.12,"w":0.8,"h":0.8,"t":0.14,"span":0.42,"spring":0.3}'),
  ('tap_glow','box','mat.glow',0,'','{"x":-0.45,"y":0.03,"z":0.08,"w":0.42,"d":0.04,"h":0.48}'),
  ('shed_posts','box','mat.sawn_timber',0,'','{"x":-1.25,"z":0.75,"w":0.1,"d":0.1,"h":1.1,"repeat":2,"dx":1.6}'),
  ('shed_roof','gable_roof','mat.pantile',0,'','{"x":-0.45,"y":1.1,"z":0.5,"w":1.8,"d":0.9,"pitch":35,"overhang":0.1,"t":0.06}'),
  ('shed_cap','gable_roof','mat.cloth',1,'','{"x":-0.45,"y":1.306,"z":0.5,"w":2,"d":0.52,"pitch":35,"overhang":0,"t":0.03}'),
  ('bellows_house','box','mat.brick',0,'','{"x":1.05,"z":-0.85,"w":0.9,"d":1.1,"h":1}'),
  ('bellows_roof','gable_roof','mat.pantile',0,'','{"x":1.05,"y":1,"z":-0.85,"yaw":90,"w":1.1,"d":0.9,"pitch":38,"overhang":0.08,"t":0.06}'),
  ('bellows_cap','gable_roof','mat.cloth',1,'','{"x":1.05,"y":1.287,"z":-0.85,"yaw":90,"w":1.26,"d":0.36,"pitch":38,"overhang":0,"t":0.03}'),
  ('wheel','cylinder','mat.timber',0,'','{"x":1.6,"y":0.65,"z":-0.85,"w":1.2,"h":0.18,"segments":12,"roll":90}'),
  ('wheel_hub','cylinder','mat.iron',0,'','{"x":1.55,"y":0.65,"z":-0.85,"w":0.16,"h":0.3,"segments":6,"roll":90,"far":0}'),
  ('pipe','box','mat.iron',0,'','{"x":0.45,"y":0.7,"z":-0.85,"w":0.4,"d":0.1,"h":0.1,"far":0}'),
  ('ore','cone','mat.ore',0,'','{"x":1.15,"z":0.85,"w":1.2,"h":0.62,"segments":8}'),
  ('charcoal','dome','mat.charcoal',0,'','{"x":-1.35,"z":-1.3,"w":0.9,"h":0.4,"segments":8}'),
  ('pigs','box','mat.iron',0,'','{"x":-0.45,"y":0,"z":1.05,"w":0.9,"d":0.3,"h":0.08,"tint":"#7a7e86","repeat":2,"dz":0.36}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.35,"z":1.55,"h":2.6,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.smelter_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.smelter'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.smelter_gunpowder'), (SELECT id FROM def WHERE key='mat.ashlar'), 100);
