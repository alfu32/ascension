-- unit.trebuchet@medieval: Trebuchet (mesh.trebuchet_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.trebuchet_medieval','Trebuchet');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.trebuchet_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beams','box','mat.oak',0,'','{"x":-0.36,"y":0.22,"w":0.14,"d":1.7,"h":0.14,"repeat":2,"dx":0.72}'),
  ('ties','box','mat.oak',0,'','{"y":0.22,"z":-0.7,"w":0.86,"d":0.12,"h":0.12,"repeat":3,"dz":0.7,"far":0}'),
  ('wheels_w','wheel','mat.oak',0,'','{"x":-0.5,"z":-0.6,"w":0.44,"t":0.08,"spokes":2,"segments":8,"repeat":2,"dz":1.2}'),
  ('wheels_e','wheel','mat.oak',0,'','{"x":0.5,"z":-0.6,"w":0.44,"t":0.08,"spokes":2,"segments":8,"repeat":2,"dz":1.2}'),
  ('legs_f','box','mat.oak',0,'','{"x":-0.36,"y":0.36,"z":0.6,"w":0.1,"d":0.1,"h":1.42,"pitch":-23,"repeat":2,"dx":0.72}'),
  ('legs_b','box','mat.oak',0,'','{"x":-0.36,"y":0.36,"z":-0.6,"w":0.1,"d":0.1,"h":1.42,"pitch":23,"repeat":2,"dx":0.72,"far":0}'),
  ('axle','box','mat.oak',0,'','{"y":1.64,"w":0.86,"d":0.12,"h":0.12,"far":0}'),
  ('hub','box','mat.iron',0,'','{"y":1.7,"w":0.16,"d":0.16,"h":0.16,"strike":170}'),
  ('arm','box','mat.sawn_timber',0,'hub','{"w":0.11,"d":0.11,"h":2.1,"pitch":-142}'),
  ('short_arm','box','mat.sawn_timber',0,'hub','{"w":0.12,"d":0.12,"h":0.5,"pitch":40,"far":0}'),
  ('hinge','box','mat.iron',0,'short_arm','{"y":0.5,"w":0.3,"d":0.06,"h":0.06,"pitch":-40,"strike":-170}'),
  ('counterweight','box','mat.oak',0,'hinge','{"y":-0.48,"w":0.42,"d":0.4,"h":0.42,"taper":0.15}'),
  ('weight_cloth','box','mat.cloth',1,'hinge','{"y":-0.4,"w":0.44,"d":0.42,"h":0.3,"far":0}'),
  ('sling','cylinder','mat.thatch',0,'arm','{"y":2.06,"w":0.025,"h":0.5,"segments":3,"pitch":150,"strike":50,"far":0}'),
  ('pouch','box','mat.leather',0,'sling','{"y":0.5,"w":0.12,"d":0.12,"h":0.1}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.44,"z":-0.8,"h":1.95,"fw":0.5,"fh":0.7}'),
  ('pennons','box','mat.cloth',1,'','{"x":-0.36,"y":1.55,"z":0.12,"w":0.02,"d":0.34,"h":0.24,"repeat":2,"dx":0.72,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.trebuchet_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.trebuchet'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.trebuchet_medieval'), (SELECT id FROM def WHERE key='mat.oak'), 100);
