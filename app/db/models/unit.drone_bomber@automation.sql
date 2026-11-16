-- unit.drone_bomber@automation: Drone bomber (mesh.drone_bomber_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.drone_bomber_automation','Drone bomber');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.drone_bomber_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('centre','box','mat.graphite',0,'','{"y":4.3,"w":0.7,"d":1.6,"h":0.3,"taper":0.5,"bank":10}'),
  ('wing_e','box','mat.graphite',0,'centre','{"y":0.04,"w":0.06,"d":1.5,"h":2.2,"taper":0.8,"roll":90,"yaw":35}'),
  ('wing_w','box','mat.graphite',0,'centre','{"y":0.04,"w":0.06,"d":1.5,"h":2.2,"taper":0.8,"roll":-90,"yaw":-35}'),
  ('tip_e','box','mat.graphite',1,'wing_e','{"y":1.55,"w":0.065,"d":0.62,"h":0.65,"taper":0.4}'),
  ('tip_w','box','mat.graphite',1,'wing_w','{"y":1.55,"w":0.065,"d":0.62,"h":0.65,"taper":0.4}'),
  ('hump','box','mat.graphite',0,'centre','{"y":0.28,"z":0.15,"w":0.42,"d":0.8,"h":0.12,"taper":0.5}'),
  ('stripe','box','mat.graphite',1,'centre','{"y":0.28,"z":-0.42,"w":0.36,"d":0.3,"h":0.04}'),
  ('edge','box','mat.light_strip',0,'centre','{"y":0.12,"z":0.66,"w":0.36,"d":0.02,"h":0.03,"far":0}'),
  ('exhausts','wheel','mat.glow',0,'centre','{"x":-0.18,"y":0.06,"z":-0.82,"yaw":90,"w":0.14,"t":0.03,"spokes":3,"segments":6,"repeat":2,"dx":0.36}'),
  ('bay','box','mat.composite',0,'centre','{"y":-0.02,"z":0.1,"w":0.3,"d":0.8,"h":0.02,"far":0}'),
  ('bomb_drop','box','mat.flash',0,'centre','{"y":-0.02,"z":0.1,"w":0.02,"d":0.02,"h":0.02,"flash":1,"far":0}'),
  ('bombs','cylinder','mat.composite',0,'bomb_drop','{"y":-0.5,"z":-0.3,"w":0.12,"h":0.3,"segments":5,"pitch":90,"repeat":3,"dz":0.3,"far":0}'),
  ('bombs_low','cylinder','mat.composite',0,'bomb_drop','{"y":-1.3,"z":-0.15,"w":0.12,"h":0.3,"segments":5,"pitch":90,"repeat":2,"dz":0.36,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.drone_bomber_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.drone_bomber'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.drone_bomber_automation'), (SELECT id FROM def WHERE key='mat.graphite'), 100);
