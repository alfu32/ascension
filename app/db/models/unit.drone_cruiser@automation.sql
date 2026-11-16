-- unit.drone_cruiser@automation: Drone cruiser (mesh.drone_cruiser_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.drone_cruiser_automation','Drone cruiser');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.drone_cruiser_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.graphite',0,'','{"yaw":-90,"y":-0.18,"w":4.4,"d":0.95,"h":0.6,"sheer":0.08,"segments":12,"float":1}'),
  ('deck','box','mat.tarmac',0,'hull','{"y":0.54,"w":3.3,"d":0.88,"h":0.04}'),
  ('boot','box','mat.white_composite',1,'hull','{"y":0.42,"z":-0.45,"w":3,"d":0.03,"h":0.1,"repeat":2,"dz":0.9,"far":0}'),
  ('superstructure','box','mat.white_composite',0,'hull','{"y":0.56,"w":1.6,"d":0.8,"h":0.55,"taper":0.35}'),
  ('base_band','box','mat.white_composite',1,'hull','{"y":0.58,"w":1.6,"d":0.8,"h":0.1}'),
  ('band','box','mat.light_strip',0,'hull','{"y":1,"w":1.17,"d":0.59,"h":0.03,"far":0}'),
  ('mast','box','mat.white_composite',0,'hull','{"x":0.15,"y":1.11,"w":0.5,"d":0.5,"h":0.6,"taper":0.55}'),
  ('mast_band','box','mat.white_composite',1,'hull','{"x":0.15,"y":1.3,"w":0.42,"d":0.42,"h":0.12}'),
  ('dome','dome','mat.white_composite',0,'hull','{"x":0.15,"y":1.7,"w":0.26,"h":0.22,"segments":7}'),
  ('cells_fore','box','mat.graphite',0,'hull','{"x":1.18,"y":0.56,"w":0.56,"d":0.56,"h":0.04}'),
  ('hatches_fore','box','mat.composite',0,'hull','{"x":0.98,"y":0.6,"z":-0.17,"w":0.1,"d":0.1,"h":0.015,"repeat":4,"dx":0.13,"far":0}'),
  ('hatches_fore_2','box','mat.composite',0,'hull','{"x":0.98,"y":0.6,"z":0.17,"w":0.1,"d":0.1,"h":0.015,"repeat":4,"dx":0.13,"far":0}'),
  ('cells_aft','box','mat.graphite',0,'hull','{"x":-0.95,"y":0.56,"w":0.5,"d":0.56,"h":0.04,"far":0}'),
  ('gun_house','box','mat.white_composite',0,'hull','{"yaw":90,"x":1.72,"y":0.58,"w":0.32,"d":0.4,"h":0.18,"taper":0.5,"traverse":30}'),
  ('gun','cylinder','mat.white_composite',0,'gun_house','{"y":0.07,"z":0.14,"w":0.05,"h":0.52,"segments":5,"pitch":90,"recoil":0.06}'),
  ('pad','cylinder','mat.white_composite',1,'hull','{"x":-1.62,"y":0.58,"w":0.56,"h":0.01,"segments":10}'),
  ('pad_drone','box','mat.composite',0,'hull','{"x":-1.62,"y":0.59,"w":0.2,"d":0.2,"h":0.06,"yaw":45,"far":0}'),
  ('ensign','box','mat.white_composite',1,'hull','{"x":-2.1,"y":0.98,"w":0.3,"d":0.015,"h":0.2,"far":0}'),
  ('ensign_staff','cylinder','mat.graphite',0,'hull','{"x":-1.98,"y":0.58,"w":0.03,"h":0.6,"segments":4,"far":0}'),
  ('spark','cone','mat.flash',0,'cells_fore','{"y":0.04,"w":0.144,"h":0.144,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.384,"w":0.432,"h":0.384,"segments":6,"pitch":180,"far":0}'),
  ('missile','cylinder','mat.white_metal',0,'spark','{"y":1.2,"w":0.084,"h":0.36,"segments":5,"far":0}'),
  ('trail','cone','mat.canvas',0,'missile','{"y":-1.08,"w":0.24,"h":1.1,"segments":5,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.drone_cruiser_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.drone_cruiser'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.drone_cruiser_automation'), (SELECT id FROM def WHERE key='mat.graphite'), 100);
